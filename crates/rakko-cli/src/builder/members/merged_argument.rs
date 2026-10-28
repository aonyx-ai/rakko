use rakko_action::{Argument, ArgumentShape, Documentation, Name};

/// One flag of a group, with every member that declares its argument
///
/// The members of a group can declare the same argument, and the group then
/// carries one flag for it. The flag takes its name and its shape from the
/// first member that declares the argument, and every later member has to
/// declare the same shape. The help of the flag comes from all of them,
/// because each member describes the argument in a sentence of its own.
pub(crate) struct MergedArgument {
    /// The member that declares the argument first
    owner: Name,
    /// The argument as the first member describes it
    argument: Argument,
    /// The members that declare the argument, each with its sentence, in the
    /// order of the members
    declarations: Vec<(Name, Documentation)>,
}

impl MergedArgument {
    /// Creates the flag of an argument that a member declares first
    pub(crate) fn new(member: Name, argument: &Argument) -> Self {
        Self {
            owner: member.clone(),
            argument: argument.clone(),
            declarations: vec![(member, argument.documentation().clone())],
        }
    }

    /// Adds a later member that declares the same argument
    ///
    /// # Panics
    ///
    /// Panics when the member declares the argument with a shape other than
    /// the shape of the first member, and reports the group, both members,
    /// the argument, and both shapes.
    pub(crate) fn add(&mut self, group: &Name, member: Name, argument: &Argument) {
        assert!(
            self.argument.shape() == argument.shape(),
            "the group '{group}' cannot merge the argument '{}', because the action '{}' declares \
             it as {} and the action '{member}' declares it as {}",
            argument.name(),
            self.owner,
            describe(*self.argument.shape()),
            describe(*argument.shape()),
        );

        self.declarations
            .push((member, argument.documentation().clone()));
    }

    /// Returns the argument as the first member describes it
    ///
    /// The name and the shape hold for every member that declares the
    /// argument. The documentation is the sentence of the first member only.
    pub(crate) fn argument(&self) -> &Argument {
        &self.argument
    }

    /// Returns the help of the flag
    ///
    /// The help holds one line for each member that declares the argument, in
    /// the order of the members. Each line starts with the name of the
    /// member, because several members can share one sentence, and without
    /// the names the help would repeat a line that a reader cannot place.
    pub(crate) fn help(&self) -> String {
        self.declarations
            .iter()
            .map(|(member, documentation)| format!("{member}: {documentation}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Returns the words with which a message describes a shape
fn describe(shape: ArgumentShape) -> &'static str {
    match shape {
        ArgumentShape::Boolean => "a boolean",
        ArgumentShape::Integer => "a whole number",
        ArgumentShape::Path => "a path",
        ArgumentShape::Text => "a text",
    }
}
