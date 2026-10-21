use rakko_action::{
    Access, ArgsValues, BoundAction, Context, ErasedAction, Name, Outcome, ReadArgsError,
};

/// One action of a run, with the values of the run bound to its arguments
///
/// The command orders the run from the declaration of each action. An action
/// declares its access only once its arguments are bound, so the command binds
/// every action before the first one runs. The bind can fail. Then the step
/// keeps the error, and the run reports it as the outcome of the action in the
/// place where the action would have run.
pub(crate) struct Step<'a> {
    /// The name of the action, which the report of the step carries
    name: Name,

    /// The action with its arguments, or the error of the bind
    binding: Result<Box<dyn BoundAction + 'a>, ReadArgsError>,
}

impl<'a> Step<'a> {
    /// Binds the values of the run to the arguments of the action
    pub(crate) fn bind(action: &'a dyn ErasedAction, values: &ArgsValues) -> Self {
        Self {
            name: action.name(),
            binding: action.bind(values),
        }
    }

    /// Returns the name of the action
    pub(crate) fn name(&self) -> Name {
        self.name.clone()
    }

    /// Returns how the step uses the project
    ///
    /// An action whose values did not convert declares nothing. So the step
    /// counts as a write, which is the answer of an action that declares
    /// nothing.
    pub(crate) fn access(&self, context: &Context) -> Access {
        match &self.binding {
            Ok(bound) => bound.access(context),
            Err(_) => Access::Write,
        }
    }

    /// Runs the action and returns its outcome
    ///
    /// A step whose values did not convert runs nothing, and its outcome
    /// carries the error of the bind.
    pub(crate) async fn run(self, context: &Context) -> Outcome {
        match self.binding {
            Ok(bound) => bound.run(context).await,
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}
