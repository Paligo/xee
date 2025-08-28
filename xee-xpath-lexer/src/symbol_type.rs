use crate::lexer::Token;

pub(crate) enum SymbolType {
    Delimiting,
    NonDelimiting,
    Whitespace,
    CommentStart,
    Error,
}

impl Token<'_> {
    pub(crate) fn symbol_type(&self) -> SymbolType {
        use crate::lexer::Token::*;
        match self {
            // A.2.2 terminal delimination
            // delimiting terminal symbols
            ExclamationMark | NotEqual | StringLiteral(_) | Hash | Dollar | LeftParen
            | RightParen | Asterisk | AsteriskColon | Plus | Comma | Minus | Dot | DotDot
            | Slash | DoubleSlash | Colon | ColonAsterisk | DoubleColon | ColonEqual | LessThan
            | Precedes | LessThanEqual | Equal | Arrow | GreaterThan | GreaterThanEqual
            | Follows | QuestionMark | At | BracedURILiteral(_) | LeftBracket | RightBracket
            | LeftBrace | Pipe | DoublePipe | RightBrace |
            // starts with *: so is delimiting
            PrefixWildcard(_) |
            // starts with BracedURILiteral so is delimiting
            BracedURILiteralWildcard(_)=> {
                SymbolType::Delimiting
            }

            // non-delimiting terminal symbols
            IntegerLiteral(_)
            | NCName(_)
            | PrefixedQName(_)
            | URIQualifiedName(_)
            // starts with a ncname, so is non-delimiting
            | LocalNameWildcard(_)
            | DecimalLiteral(_)
            | DoubleLiteral(_)
            | Ancestor
            | AncestorOrSelf
            | And
            | Array
            | As
            | Attribute
            | Cast
            | Castable
            | Child
            | Comment
            | Descendant
            | DescendantOrSelf
            | Div
            | DocumentNode
            | Element
            | Else
            | EmptySequence
            | Eq
            | Every
            | Except
            | Following
            | FollowingSibling
            | For
            | Function
            | Ge
            | Gt
            | Idiv
            | If
            | In
            | Instance
            | Intersect
            | Is
            | Item
            | Le
            | Let
            | Lt
            | Map
            | Mod
            | Namespace
            | NamespaceNode
            | Ne
            | Node
            | Of
            | Or
            | Parent
            | Preceding
            | PrecedingSibling
            | ProcessingInstruction
            | Return
            | Satisfies
            | SchemaAttribute
            | SchemaElement
            | Self_
            | Some
            | Text
            | Then
            | To
            | Treat
            | Union
            | Switch
            | Typeswitch => SymbolType::NonDelimiting,


            Token::Whitespace => SymbolType::Whitespace,
            Token::CommentStart => SymbolType::CommentStart,
            Token::Error => SymbolType::Error,
        }
    }
}
