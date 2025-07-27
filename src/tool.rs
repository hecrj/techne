use crate::content;
use crate::{Content, Notification, Request, Schema};

use futures::SinkExt;
use futures::channel::mpsc;
use serde::{Deserialize, Serialize};
use tokio::task;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::error::Error;
use std::io;
use std::marker::PhantomData;

pub struct Tool<Name = Cow<'static, str>, Description = Cow<'static, str>> {
    pub name: Name,
    pub description: Description,
    input: Schema,
    output: Option<Schema>,
    call: Box<dyn Fn(serde_json::Value) -> io::Result<mpsc::Receiver<Action>> + Send + Sync>,
}

pub enum Action {
    Request(Request),
    Notify(Notification),
    Finish(io::Result<Outcome>),
}

impl Tool<(), ()> {
    /// # Safety
    /// The input and output schemas must match the `call` implementation.
    pub unsafe fn new(
        input: Schema,
        output: Option<Schema>,
        call: impl Fn(serde_json::Value) -> io::Result<mpsc::Receiver<Action>> + Send + Sync + 'static,
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
    pub fn name(self, name: impl Into<Cow<'static, str>>) -> Tool<Cow<'static, str>, Description> {
        Tool {
            name: name.into(),
            description: self.description,
            input: self.input,
            output: self.output,
            call: self.call,
        }
    }

    pub fn description(self, description: impl Into<Cow<'static, str>>) -> Tool<Name> {
        Tool {
            name: self.name,
            description: description.into(),
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

    pub fn call(&self, json: serde_json::Value) -> io::Result<mpsc::Receiver<Action>> {
        (self.call)(json)
    }
}

pub fn tool<A, O, F>(
    f: impl Fn(A) -> F + Send + Sync + 'static,
    a: impl Argument<A> + Send + Sync + 'static,
) -> Tool<(), ()>
where
    O: IntoOutcome,
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
    O: IntoOutcome,
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
    O: IntoOutcome,
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

    fn deserialize(&self, json: serde_json::Value) -> serde_json::Result<T>;

    fn is_required(&self) -> bool {
        true
    }
}

pub fn string(
    name: impl Into<Cow<'static, str>>,
    description: impl Into<Cow<'static, str>>,
) -> impl Argument<String> {
    NamedArg::new(name, description)
}

pub fn u32(
    name: impl Into<Cow<'static, str>>,
    description: impl Into<Cow<'static, str>>,
) -> impl Argument<u32> {
    NamedArg::new(name, description)
}

pub fn f32(
    name: impl Into<Cow<'static, str>>,
    description: impl Into<Cow<'static, str>>,
) -> impl Argument<f32> {
    NamedArg::new(name, description)
}

pub fn bool(
    name: impl Into<Cow<'static, str>>,
    description: impl Into<Cow<'static, str>>,
) -> impl Argument<bool> {
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

        fn deserialize(&self, json: serde_json::Value) -> serde_json::Result<Option<T>> {
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
    name: Cow<'static, str>,
    description: Cow<'static, str>,
}

impl NamedArg {
    fn new(name: impl Into<Cow<'static, str>>, description: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

impl Argument<String> for NamedArg {
    fn name(&self) -> &str {
        &self.name
    }

    fn schema(&self) -> Schema {
        Schema::String {
            description: Some(self.description.clone().into_owned()),
        }
    }

    fn deserialize(&self, json: serde_json::Value) -> serde_json::Result<String> {
        serde_json::from_value(json)
    }
}

impl Argument<u32> for NamedArg {
    fn name(&self) -> &str {
        &self.name
    }

    fn schema(&self) -> Schema {
        Schema::Integer {
            description: Some(self.description.clone().into_owned()),
        }
    }

    fn deserialize(&self, json: serde_json::Value) -> serde_json::Result<u32> {
        serde_json::from_value(json)
    }
}

impl Argument<f32> for NamedArg {
    fn name(&self) -> &str {
        &self.name
    }

    fn schema(&self) -> Schema {
        Schema::Number {
            description: Some(self.description.clone().into_owned()),
        }
    }

    fn deserialize(&self, json: serde_json::Value) -> serde_json::Result<f32> {
        serde_json::from_value(json)
    }
}

impl Argument<bool> for NamedArg {
    fn name(&self) -> &str {
        &self.name
    }

    fn schema(&self) -> Schema {
        Schema::Boolean {
            description: Some(self.description.clone().into_owned()),
        }
    }

    fn deserialize(&self, json: serde_json::Value) -> serde_json::Result<bool> {
        serde_json::from_value(json)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome<T = serde_json::Value> {
    #[serde(flatten)]
    content: Content<T>,
    is_error: bool,
}

impl<T> Outcome<T> {
    pub async fn serialize(self) -> serde_json::Result<Outcome>
    where
        T: Serialize,
    {
        Ok(Outcome {
            content: match self.content {
                Content::Unstructured(content) => Content::Unstructured(content),
                Content::Structured(content) => {
                    Content::Structured(serde_json::to_value(&content)?)
                }
            },
            is_error: self.is_error,
        })
    }
}

pub trait IntoOutcome {
    type Content;

    fn into_outcome(self) -> Outcome<Self::Content>;
}

impl<T> IntoOutcome for T
where
    T: Into<Content<T>>,
{
    type Content = T;

    fn into_outcome(self) -> Outcome<Self::Content> {
        Outcome {
            content: self.into(),
            is_error: false,
        }
    }
}

impl<T, E> IntoOutcome for Result<T, E>
where
    T: Into<Content<T>>,
    E: Error,
{
    type Content = T;

    fn into_outcome(self) -> Outcome<T> {
        match self {
            Ok(value) => Outcome {
                content: value.into(),
                is_error: false,
            },
            Err(error) => Outcome {
                content: Content::Unstructured(vec![content::Unstructured::Text {
                    text: error.to_string(),
                }]),
                is_error: false,
            },
        }
    }
}

fn property<T>(arg: &impl Argument<T>) -> (String, Schema) {
    (arg.name().to_owned(), arg.schema().clone())
}

fn required<T>(arg: &impl Argument<T>) -> Option<String> {
    arg.is_required().then(|| arg.name().to_owned())
}

fn object(
    json: serde_json::Value,
) -> serde_json::Result<serde_json::Map<String, serde_json::Value>> {
    serde_json::from_value(json)
}

fn deserialize<T>(
    arg: &impl Argument<T>,
    object: &mut serde_json::Map<String, serde_json::Value>,
) -> serde_json::Result<T> {
    arg.deserialize(object.remove(arg.name()).unwrap_or(serde_json::Value::Null))
}
