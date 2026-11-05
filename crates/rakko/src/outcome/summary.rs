typed_fields::name! {
    /// A sentence that tells what a passing run examined
    ///
    /// "Passed" alone reads the same when an action checked the whole project
    /// and when its tool matched nothing at all. The summary gives the reader
    /// the scope of the pass, such as `checked 3 files`, so a pass that
    /// examined less than the reader expects points to a misconfiguration
    /// instead of hiding it.
    Summary
}
