use crate::mcp;
use crate::mcp::server::tool::{IntoResponse, Response};
use crate::mcp::server::{Notification, Request};
use crate::mcp::{Map, Schema, Value};

use futures::SinkExt;
use futures::channel::mpsc;
use serde::Serialize;
use tokio::task;

use std::collections::BTreeMap;
use std::io;
use std::marker::PhantomData;

pub struct Tool<Name = String, Description = String> {
    pub name: Name,
    pub description: Description,
    input: Schema,
    output: Option<Schema>,
    call: Box<dyn Fn(Value) -> io::Result<mpsc::Receiver<Action>> + Send + Sync>,
}

pub enum Action {
    Request(Request),
    Notify(Notification),
    Finish(io::Result<Response>),
}

impl Tool<(), ()> {
    /// # Safety
    /// The input and output schemas must match the `call` implementation.
    pub unsafe fn new(
        input: Schema,
        output: Option<Schema>,
        call: impl Fn(Value) -> io::Result<mpsc::Receiver<Action>> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: (),
            description: (),
            input,
            output,
            call: Box::new(call),
        }
    }
}

impl<Name, Description> Tool<Name, Description> {
    pub fn name(self, name: impl AsRef<str>) -> Tool<String, Description> {
        Tool {
            name: name.as_ref().to_owned(),
            description: self.description,
            input: self.input,
            output: self.output,
            call: self.call,
        }
    }

    pub fn description(self, description: impl AsRef<str>) -> Tool<Name> {
        Tool {
            name: self.name,
            description: description.as_ref().to_owned(),
            input: self.input,
            output: self.output,
            call: self.call,
        }
    }
}

impl Tool {
    pub fn input(&self) -> &Schema {
        &self.input
    }

    pub fn output(&self) -> Option<&Schema> {
        self.output.as_ref()
    }

    pub fn call(&self, json: Value) -> io::Result<mpsc::Receiver<Action>> {
        (self.call)(json)
    }
}

pub fn tool<A, O, F>(
    f: impl Fn(A) -> F + Send + Sync + 'static,
    a: impl Argument<A> + Send + Sync + 'static,
) -> Tool<(), ()>
where
    O: IntoResponse,
    O::Content: Serialize + Send,
    F: Future<Output = O> + Send + 'static,
{
    let input = Schema::Object {
        description: None,
        properties: BTreeMap::from_iter([property(&a)]),
        required: Vec::from_iter([required(&a)].into_iter().flatten()),
    };

    let call = move |json| {
        let mut object = object(json)?;
        let a = deserialize(&a, &mut object)?;

        Ok(spawn(f(a)))
    };

    Tool {
        name: (),
        description: (),
        input,
        output: None, // TODO
        call: Box::new(call),
    }
}

pub fn tool_2<A, B, O, F>(
    f: impl Fn(A, B) -> F + Send + Sync + 'static,
    a: impl Argument<A> + Send + Sync + 'static,
    b: impl Argument<B> + Send + Sync + 'static,
) -> Tool<(), ()>
where
    O: IntoResponse,
    O::Content: Serialize + Send,
    F: Future<Output = O> + Send + 'static,
{
    let input = Schema::Object {
        description: None,
        properties: BTreeMap::from_iter([property(&a), property(&b)]),
        required: Vec::from_iter([required(&a), required(&b)].into_iter().flatten()),
    };

    let call = move |json| {
        let mut object = object(json)?;
        let a = deserialize(&a, &mut object)?;
        let b = deserialize(&b, &mut object)?;

        Ok(spawn(f(a, b)))
    };

    Tool {
        name: (),
        description: (),
        input,
        output: None, // TODO
        call: Box::new(call),
    }
}

fn spawn<O>(execution: impl Future<Output = O> + Send + 'static) -> mpsc::Receiver<Action>
where
    O: IntoResponse,
    O::Content: Serialize + Send,
{
    let (mut sender, receiver) = mpsc::channel(1);

    task::spawn(async move {
        let output = execution.await;

        let result = output
            .into_outcome()
            .serialize()
            .await
            .map_err(io::Error::from);

        let _ = sender.send(Action::Finish(result)).await;
    });

    receiver
}

pub trait Argument<T> {
    fn name(&self) -> &str;

    fn schema(&self) -> Schema;

    fn deserialize(&self, json: Value) -> io::Result<T>;

    fn is_required(&self) -> bool {
        true
    }
}

pub fn string(name: impl AsRef<str>, description: impl AsRef<str>) -> impl Argument<String> {
    NamedArg::new(name, description)
}

pub fn u32(name: impl AsRef<str>, description: impl AsRef<str>) -> impl Argument<u32> {
    NamedArg::new(name, description)
}

pub fn f32(name: impl AsRef<str>, description: impl AsRef<str>) -> impl Argument<f32> {
    NamedArg::new(name, description)
}

pub fn bool(name: impl AsRef<str>, description: impl AsRef<str>) -> impl Argument<bool> {
    NamedArg::new(name, description)
}

pub fn optional<T>(argument: impl Argument<T>) -> impl Argument<Option<T>> {
    struct Optional<A, T> {
        argument: A,
        _output: PhantomData<T>,
    }

    impl<A, T> Argument<Option<T>> for Optional<A, T>
    where
        A: Argument<T>,
    {
        fn name(&self) -> &str {
            self.argument.name()
        }

        fn schema(&self) -> Schema {
            self.argument.schema()
        }

        fn deserialize(&self, json: Value) -> io::Result<Option<T>> {
            if json.is_null() {
                return Ok(None);
            }

            self.argument.deserialize(json).map(Some)
        }

        fn is_required(&self) -> bool {
            false
        }
    }

    Optional {
        argument,
        _output: PhantomData,
    }
}

struct NamedArg {
    name: String,
    description: String,
}

impl NamedArg {
    fn new(name: impl AsRef<str>, description: impl AsRef<str>) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            description: description.as_ref().to_owned(),
        }
    }
}

impl Argument<String> for NamedArg {
    fn name(&self) -> &str {
        &self.name
    }

    fn schema(&self) -> Schema {
        Schema::String {
            description: Some(self.description.clone()),
        }
    }

    fn deserialize(&self, json: Value) -> io::Result<String> {
        mcp::from_value(json)
    }
}

impl Argument<u32> for NamedArg {
    fn name(&self) -> &str {
        &self.name
    }

    fn schema(&self) -> Schema {
        Schema::Integer {
            description: Some(self.description.clone()),
        }
    }

    fn deserialize(&self, json: Value) -> io::Result<u32> {
        mcp::from_value(json)
    }
}

impl Argument<f32> for NamedArg {
    fn name(&self) -> &str {
        &self.name
    }

    fn schema(&self) -> Schema {
        Schema::Number {
            description: Some(self.description.clone()),
        }
    }

    fn deserialize(&self, json: Value) -> io::Result<f32> {
        mcp::from_value(json)
    }
}

impl Argument<bool> for NamedArg {
    fn name(&self) -> &str {
        &self.name
    }

    fn schema(&self) -> Schema {
        Schema::Boolean {
            description: Some(self.description.clone()),
        }
    }

    fn deserialize(&self, json: Value) -> io::Result<bool> {
        mcp::from_value(json)
    }
}

fn property<T>(arg: &impl Argument<T>) -> (String, Schema) {
    (arg.name().to_owned(), arg.schema().clone())
}

fn required<T>(arg: &impl Argument<T>) -> Option<String> {
    arg.is_required().then(|| arg.name().to_owned())
}

fn object(json: Value) -> io::Result<Map<String, Value>> {
    mcp::from_value(json)
}

fn deserialize<T>(arg: &impl Argument<T>, object: &mut Map<String, Value>) -> io::Result<T> {
    arg.deserialize(object.remove(arg.name()).unwrap_or(Value::Null))
}
