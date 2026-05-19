/// XSLT has a number of things that can be declared globally, such
/// as global variables, parameters, functions, and templates. This
/// contains the runtime information to execute XSLT.
mod decl;
mod globalvar;

pub use decl::{
    Declarations, GlobalVariableDeclaration, ModeDeclaration, ModeOnNoMatch, ModeTyped,
    NamedTemplateDeclaration, TemplateParamDeclaration,
};
