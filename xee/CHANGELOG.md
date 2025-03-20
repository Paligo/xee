# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Paligo/xee/releases/tag/xee-v0.1.0) - 2025-03-20

### Other

- Update all the licenses to MIT.
- Update ariadne.
- Update some xee cli dependencies.
- id support.
- Explain this better.
- Preparing licenses, attribution, etc.
- Use capacity
- Add some example documents.
- Better error handling for loading a file. Now commands need to handle their own errors.
- Improve command error handling.
- These things are really infallible.
- When an xpath evaluation error happens we now display the output.
- Update to new Xot. Use this to display better parse errors.
- Factor out error handling into common module.
- Use the interpreter's way to get a representation now.
- Shortcut name.
- Pull command story into its own module.
- More information about the repl.
- Better representation of various things.
- Clean up.
- Start using improved display representation.
- Provide more repl commands to manipulate namespace stuff.
- Try to implement new definitions, run into lifetime issues.
- Add a command system for repl.
- Improve repl.
- A simple repl.
- A bit more test data.
- Use xee-xpath to do the querying.
- extract xpath into its own module.
- Add a few todo items.
- Add xee indent as a shortcut for xee format.
- Add a second example file for now.
- Didn't end up using this.
- Implement a format function that exposes a lot of serialization options.
- Add indent cli tool.
- Make span optional in SpannedError. This makes it a bit more convenient to use it in the outer layer.
- Make it so that the various iterators return an error early if the data is absent.
- Rename xee-xpath to xee-xpath-compiler so we can better hide information from the public APIs.
- Update to newer version of Xot.
- Convert to use Xot's OwnedName.
- Port over to new xot.
- Make xslt-ast work again.
- Add basic readmes for the various sub-crates.
- Don't rely on xee-interpreter if we already have xee-xpath as a dependency.
- Rename xee-xpath-outer to the new xee-xpath
- Rename old xee-xpath to xee-interpreter.
- Clean up imports to use indirect imports only.
- Finish extracting main behavior into xee-xpath-outer (to be xee-xpath).
- Less repetitive error messages.
- Use ariadne for error reporting.
- Add an explicit SpannedResult
- Get rid of Miette entirely.
- Further progress in removing Miette
- Reorganize matters so that we classify Map and Array as a string.
- Add Map and Array to Item. Make sure we can atomize an array.
- Rename stack::Item and output::Item to sequence::Item
- :Item wasn't pulling its weight. Collapse it into stack::Item
- Move Absent from atomic into stack value
- .iter() becomes .items()
- We rewrote outcome to outcome2, which is in terms of the stack.
- Now use output as a namespace, so we don't have Output* names anymore.
- Make OutputSequence an explicit struct
- Cleanup
- Much work to simplify the API of xee-xpath to be in terms of OutputItem.
- Create a new Sequence abstraction to hide the Rc/RefCell details
- Rename StackValue to Value
- Work towards a xee-qt cli
- We can add the default namespace.
- Fancier example.
- Better display for xee xpath commands.
- Make subcommand mandatory
- Start of simple xee tool that lets us evaluate xpath
