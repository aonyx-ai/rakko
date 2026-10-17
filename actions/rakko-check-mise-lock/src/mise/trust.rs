/// Who trusts the configuration that a run of mise reads
///
/// Mise reads a configuration that needs trust, such as one with settings,
/// only after somebody trusted the directory that holds it. The project of a
/// contributor is trusted already, and a copy of it is a directory that
/// nobody has seen before.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Trust {
    /// The run of mise trusts what the environment of the action trusts
    Inherited,

    /// The run of mise trusts the directory that it runs in
    ///
    /// The directory takes the place of any list of trusted directories that
    /// the environment of the action names, for that one run. The trust lives
    /// in the environment of the run, so no record of it remains when the run
    /// ends.
    ///
    /// The run skips the question whether anybody trusted the configuration
    /// of the directory. A caller grants it only for a directory whose
    /// configuration somebody trusted in another place, such as a copy of a
    /// project whose lockfiles mise already listed in the project.
    Granted,
}
