# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Paligo/xee/releases/tag/xee-ir-v0.1.0) - 2025-03-20

### Added

- Add constant folding optimization module to IR

### Other

- Add a few categories.
- Update copyright year.
- Update all the licenses to MIT.
- Remove all the apache licenses, MIT only.
- Make insta a single workspace dependency so it's easier to upgrade.
- Tweak license text.
- Preparing licenses, attribution, etc.
- Revert "feat: Add constant folding optimization module to IR"
- Move things from trait onto Sequence.
- Move conversion traits into its own module.
- Clippy.
- Rewrite to consolidated sequence.
- Make things more private, and expose as methods instead.
- Work towards better information hiding.
- Clippy.
- The dynamic context builder should be built from the program.
- No more static context ref is needed.
- Let program own static context.
- make namespaces own their strings, rather than using a lifetime which leaks through everything.
- Shut up more clippy stuff due to macros using non snake case.
- Introduce DocumentsRef and StaticContextRef to wrap Rc away from public API.
- Isolated static context and reconstruct dynamic context each time we run.
- Add the context item to the dynamic context too.
- Use a builder everywhere for dynamic context.
- Passing variables separately from the DynamicContext turns out to be misguided.
- A static context builder so it's easier to create them.
- Modify how variables are passed.
- Make it so that the various iterators return an error early if the data is absent.
- We try to move towards a DynamicContext we can live with.
- Dynamic context owns static context now.
- Wire in mode system.
- Wire up the declaration compiler.
- Extract declaration compiler. Not wired up yet.
- Rename the module.
- Rename this to FunctionCompiler. We'll get a DeclarationCompiler too.
- Make mode id be part of the system. Unfortunately we can't refer to it yet, as we need some kind of DeclarationBuilder.
- Mode declarations, which don't do anything yet.
- Handle the default mode early in AST parsing.
- ModeValue only needs to exist on the IR level.
- Refactor things so we support the various mode values.
- Prepare the ground for modes. We can't actually use modes yet or declare them.
- Priority and declaration order for rules is obeyed.
- Calculate priority now for rules.
- namespace
- Generate literal attribute nodes using xot attribute nodes now.
- Rename Root to Document.
- Port over to new xot.
- Wire in the new pattern lookup. This should work with predicates, but that's not tested yet.
- Add a way to transform a pattern into another pattern replacing predicates.
- copy is starting to take shape.
- We pass in an explicit mutable xot, and retain it in state.
- basic xsl:choose with only a single when branch.
- Towards a simpler bindings API.
- new_binding_no_span
- Move more common code to variables.
- Implement basic xsl:value-of
- Various cleanups, run tests from integration tests in xee_xslt.
- centralize compilation logic into xee_ir.
- Properly go through the IR while compiling XSLT.
- Pull variable handling into shared ir crate.
- Start to weave through apply-templates support
- Some thinking about global variables.
- A faltering step towards compiling XSLT.
- Some preparations for compiling XSL.
- Move the binding stuff into xee-ir as it's general and we want to use it for xslt too.
- Get hand-written AST right with proper variables. Use element to construct elements into isolated root.
- Rename instructions to use Xml prefix for clarity.
- We can now create a basic root node.
- Work towards an implementation of the instructions. We need to do something sensible with output.
- Write a test for the whole ir transformation of XML.
- Move some more dependencies into the workspace.
- Describe what it does.
- Add more information.
- A few cleanups.
- Factor out xee-ir as it's going to be shared between xpath and xslt.
