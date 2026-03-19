use crate::Token;

impl<'a> Token<'a> {
    // tokens that can count as an ncname as a local name or as a prefix
    pub(crate) fn ncname(&self) -> Option<&'a str> {
        // Per XPath 3.1 spec (https://www.w3.org/TR/xpath-31/#terminal-symbols):
        //
        //   "Keywords in XPath 3.1 use lower-case characters and are not
        //    reserved—that is, names in XPath 3.1 expressions are allowed
        //    to be the same as language keywords, except for certain
        //    unprefixed function-names listed in A.3 Reserved Function Names."
        //
        // All keyword tokens can therefore be used as valid NCNames (prefixes
        // or local names in QNames). For example, "ex:child" is a valid
        // qualified name where "child" is the local name, not the child axis.
        match self {
            // reserved function names (XPath 3.1 spec A.3)
            Token::Array => Some("array"),
            Token::Attribute => Some("attribute"),
            Token::Comment => Some("comment"),
            Token::DocumentNode => Some("document-node"),
            Token::Element => Some("element"),
            Token::EmptySequence => Some("empty-sequence"),
            Token::Function => Some("function"),
            Token::If => Some("if"),
            Token::Item => Some("item"),
            Token::Map => Some("map"),
            Token::NamespaceNode => Some("namespace-node"),
            Token::Node => Some("node"),
            Token::ProcessingInstruction => Some("processing-instruction"),
            Token::SchemaAttribute => Some("schema-attribute"),
            Token::SchemaElement => Some("schema-element"),
            Token::Switch => Some("switch"),
            Token::Text => Some("text"),
            Token::Typeswitch => Some("typeswitch"),

            // axis names
            Token::Ancestor => Some("ancestor"),
            Token::AncestorOrSelf => Some("ancestor-or-self"),
            Token::Child => Some("child"),
            Token::Descendant => Some("descendant"),
            Token::DescendantOrSelf => Some("descendant-or-self"),
            Token::Following => Some("following"),
            Token::FollowingSibling => Some("following-sibling"),
            Token::Namespace => Some("namespace"),
            Token::Parent => Some("parent"),
            Token::Preceding => Some("preceding"),
            Token::PrecedingSibling => Some("preceding-sibling"),
            Token::Self_ => Some("self"),

            // other keywords
            Token::And => Some("and"),
            Token::As => Some("as"),
            Token::Cast => Some("cast"),
            Token::Castable => Some("castable"),
            Token::Div => Some("div"),
            Token::Else => Some("else"),
            Token::Eq => Some("eq"),
            Token::Every => Some("every"),
            Token::Except => Some("except"),
            Token::For => Some("for"),
            Token::Ge => Some("ge"),
            Token::Gt => Some("gt"),
            Token::Idiv => Some("idiv"),
            Token::In => Some("in"),
            Token::Instance => Some("instance"),
            Token::Intersect => Some("intersect"),
            Token::Is => Some("is"),
            Token::Le => Some("le"),
            Token::Let => Some("let"),
            Token::Lt => Some("lt"),
            Token::Mod => Some("mod"),
            Token::Ne => Some("ne"),
            Token::Of => Some("of"),
            Token::Or => Some("or"),
            Token::Return => Some("return"),
            Token::Satisfies => Some("satisfies"),
            Token::Some => Some("some"),
            Token::Then => Some("then"),
            Token::To => Some("to"),
            Token::Treat => Some("treat"),
            Token::Union => Some("union"),

            // an NCName of course can also be a prefix or a local name
            Token::NCName(name) => Some(name),
            _ => None,
        }
    }
}
