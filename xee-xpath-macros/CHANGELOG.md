# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Paligo/xee/releases/tag/xee-xpath-macros-v0.1.0) - 2025-03-20

### Other

- Update copyright year.
- Update all the licenses to MIT.
- Remove all the apache licenses, MIT only.
- Ignore the new errors for now, as the parser is so much faster it's worth it.
- Make insta a single workspace dependency so it's easier to upgrade.
- Tweak license text.
- Preparing licenses, attribution, etc.
- Use iterators in library.
- Move things from trait onto Sequence.
- Rewrite to consolidated sequence.
- Shut up more clippy stuff due to macros using non snake case.
- Make it so that the various iterators return an error early if the data is absent.
- Modify so that xee-xpath-ast uses xee-xpath-lexer
- Convert to use Xot's OwnedName.
- We now pass in a mutable xot everywhere.
- We pass in an explicit mutable xot, and retain it in state.
- Separate out sequence type ast so interpreter can start to depend on it alone.
- Add basic readmes for the various sub-crates.
- Rename old xee-xpath to xee-interpreter.
- Clean up imports to use indirect imports only.
- Get rid of expected found, so we can get rid of the lifetime on ParserError, which simplifies things.
- To try to integrate XPath parsing into the XSLT parser we need better error handling in XPath
- More specific errors in the xpath parser.
- A stab at sequence type subtype relations. Not finished yet.
- Parse atomic type directly into the Xs type when we construct the AST.
- Move static function stuff into the function module.
- Implement array:get
- Implement fn:function-lookup
- sort. Also add caseblind collation
- implement fn:for-each higher order function
- Implement various fn surrounding qname.
- data
- Support xs:numeric enough to implement fn:abs
- Fix macro generation some more.
- Implement contains token, tokenize.
- Fix macro handling some more, so we're properly eating the collation argument.
- Implement enough macro magic to accept a `collation` argument.
- Use xee-schema-type to generate rust type name.
- We got type promotion and things integrated now. Now only to test them.
- Move things around some more, introduce parse constructor functions on ast to make API cleaner.
- Change import signatures. Still not done moving parser around.
- Keywords can now be ncnames too. Phew.
- Fix xee-xpath-macros with new ast
- Oops, these snapshots were still out of date.
- Use ibig to store integer, to be closer to spec.
- Better IR generation for cast and castable.
- Remove original Occurrence, rename ResultOccurrence to Occurrance.
- Move Absent from atomic into stack value
- There is only one Atomic. There is only one Error
- Implement fn:string-length, fn:boolean, fn:concat
- Support strings in xpath_fn functions
- Do not borrow argument as it's not necessary.
- Rename convert2 to convert
- No more original convert.
- Rewrite the xpath_fn system to be output::Sequence based
- Implement conversion code based on output::Sequence
- Move ContextTryFrom and ContextInto into context module.
- Rename StackValue to stack::Value
- Rename ValueError and ValueResult to stack::Error and stack::Result
- Move convert into stack level
- Move ValueError and ValueResult into stack module
- Import from crate::stack where possible
- Rename Value back to StackValue.
- Try to make another type signature work.
- Make Value and Sequence be an internal thing.
- Trying to remove Value and Atomic from the API in favor of OutputItem everywhere (and OutputAtomic).
- Get rid of OutputValue as we don't use it anymore in the public API.
- xpath_fn can return ValueResult now.
- Make it so we can also pass through context information.
- No more clippy warnings.
- Implement wrap_xpath_fn! macro
- Some speculative, commented out code for generating the proper wrapper information.
- Make fn:count work with the new system.
- More advanced conversion code, so we can handle more cases.
- Optionally get context and pass it in.
- Use xpath_fn on my_function, it works!
- We can now convert incoming arguments.
- We generate a very basic wrapper with the macro now.
- A first stab at a macro.
- Start of macros crate.
