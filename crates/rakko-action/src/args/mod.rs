/// The error type for reading the arguments of a run
mod error;
/// Types for the description of an argument set
pub mod schema;
/// Types for the values that a run reads
pub mod values;

pub use self::error::ReadArgsError;
pub use self::schema::{ArgsSchema, Argument, ArgumentName, ArgumentShape, Documentation};
pub use self::values::{ArgsValues, ArgumentValue};

/// The arguments that a run of an action reads
///
/// Every action defines its own type of arguments, and that type implements
/// this trait. The trait does two things that the machinery cannot do for
/// itself: it describes the arguments as data, so that a projection can build
/// a command before any value exists, and it builds a value of the type from
/// what the machinery parsed.
///
/// Both halves have to live with the type, because only the crate that defines
/// a type can generate code from its fields. The machinery never sees the
/// type. It reads the description, collects [`ArgsValues`], and hands them
/// back, and the conversion happens here.
///
/// An action that reads no arguments uses the unit type, which implements this
/// trait for the empty argument set.
///
/// # Examples
///
/// ```
/// use rakko_action::{
///     Args, ArgsSchema, ArgsValues, Argument, ArgumentName, ArgumentShape, ReadArgsError,
/// };
///
/// struct FormatArgs {
///     fix: bool,
/// }
///
/// impl Args for FormatArgs {
///     fn schema() -> ArgsSchema {
///         ArgsSchema::new([Argument::builder()
///             .name("fix")
///             .shape(ArgumentShape::Boolean)
///             .documentation("Rewrite the files that the formatter can format")
///             .build()])
///     }
///
///     fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
///         let name = ArgumentName::new("fix");
///         let fix = match values.get(&name) {
///             Some(value) => value.get().parse().map_err(|_| ReadArgsError::UnreadableValue {
///                 name,
///                 value: value.clone(),
///             })?,
///             None => false,
///         };
///
///         Ok(Self { fix })
///     }
/// }
/// ```
// action[impl args.send]
// action[impl args.sync]
pub trait Args: Send + Sync + Sized {
    /// Returns the description of the arguments
    ///
    /// A projection calls this before a run exists, so the description belongs
    /// to the type and not to a value of it.
    // action[impl args.schema]
    fn schema() -> ArgsSchema;

    /// Builds the arguments from the values that the machinery parsed
    ///
    /// # Errors
    ///
    /// Returns a [`ReadArgsError`] when a value that the arguments require is
    /// absent, or when a value does not convert to the type of its field.
    // action[impl args.values]
    fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError>;
}

// action[impl args.empty]
impl Args for () {
    fn schema() -> ArgsSchema {
        ArgsSchema::empty()
    }

    fn from_values(_values: &ArgsValues) -> Result<Self, ReadArgsError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    /// Compiles only when the trait makes every argument set `Send`
    fn require_args_send<A: Args>() {
        assert_send::<A>();
    }

    /// Compiles only when the trait makes every argument set `Sync`
    fn require_args_sync<A: Args>() {
        assert_sync::<A>();
    }

    // action[verify args.send]
    #[test]
    fn args_are_send() {
        require_args_send::<()>();
    }

    // action[verify args.sync]
    #[test]
    fn args_are_sync() {
        require_args_sync::<()>();
    }

    // action[verify args.empty]
    #[test]
    fn unit_builds_from_values() {
        let values = ArgsValues::empty();

        let args = <() as Args>::from_values(&values);

        assert_eq!(args, Ok(()));
    }

    // action[verify args.empty]
    #[test]
    fn unit_describes_no_arguments() {
        let schema = <() as Args>::schema();

        assert!(schema.arguments().is_empty());
    }
}
