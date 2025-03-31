// https://www.w3.org/TR/xpath-31/#id-sequencetype-subtype

use crate::ast;

/// Type information for a sequence type.
pub trait TypeInfo {
    /// Get whether `self` is a subtype of `other`.
    fn subtype(&self, other: &Self) -> bool;
}

impl TypeInfo for ast::SequenceType {
    // https://www.w3.org/TR/xpath-31/#id-seqtype-subtype
    fn subtype(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::Empty, Self::Item(ast::Item { occurrence, .. })) => {
                match occurrence {
                    ast::Occurrence::Option => true,
                    ast::Occurrence::Many => true,
                    ast::Occurrence::One => false,
                    ast::Occurrence::NonEmpty => false,
                }
            }
            (Self::Item(_), Self::Empty) => false,
            (
                Self::Item(ast::Item {
                    item_type: a_item_type,
                    occurrence: a_occurrence,
                }),
                Self::Item(ast::Item {
                    item_type: b_item_type,
                    occurrence: b_occurrence,
                }),
            ) => match (a_occurrence, b_occurrence) {
                (ast::Occurrence::Option, ast::Occurrence::Option) => {
                    a_item_type.subtype(b_item_type)
                }
                (ast::Occurrence::Option, ast::Occurrence::Many) => {
                    a_item_type.subtype(b_item_type)
                }
                (ast::Occurrence::Option, ast::Occurrence::One) => false,
                (ast::Occurrence::Option, ast::Occurrence::NonEmpty) => false,
                (ast::Occurrence::Many, ast::Occurrence::Option) => false,
                (ast::Occurrence::Many, ast::Occurrence::Many) => a_item_type.subtype(b_item_type),
                (ast::Occurrence::Many, ast::Occurrence::One) => false,
                (ast::Occurrence::Many, ast::Occurrence::NonEmpty) => false,
                (ast::Occurrence::One, ast::Occurrence::Option) => a_item_type.subtype(b_item_type),
                (ast::Occurrence::One, ast::Occurrence::Many) => a_item_type.subtype(b_item_type),
                (ast::Occurrence::One, ast::Occurrence::One) => a_item_type.subtype(b_item_type),
                (ast::Occurrence::One, ast::Occurrence::NonEmpty) => {
                    a_item_type.subtype(b_item_type)
                }
                (ast::Occurrence::NonEmpty, ast::Occurrence::Option) => false,
                (ast::Occurrence::NonEmpty, ast::Occurrence::Many) => {
                    a_item_type.subtype(b_item_type)
                }
                (ast::Occurrence::NonEmpty, ast::Occurrence::One) => false,
                (ast::Occurrence::NonEmpty, ast::Occurrence::NonEmpty) => {
                    a_item_type.subtype(b_item_type)
                }
            },
        }
    }
}

impl TypeInfo for ast::ItemType {
    // https://www.w3.org/TR/xpath-31/#id-itemtype-subtype
    fn subtype(&self, other: &Self) -> bool {
        match (self, other) {
            // 1 Ai and Bi are AtomicOrUnionTypes, and derives-from(Ai, Bi)
            // returns true.
            (Self::AtomicOrUnionType(a), Self::AtomicOrUnionType(b)) => {
                a.derives_from(*b)
            }
            // 2 TODO Ai is a pure union type, and every type t in the
            // transitive membership of Ai satisfies subtype-itemType(t, Bi).

            // 3 TODO Ai is xs:error and Bi is a generalized atomic type. 4 Bi
            // is item()
            (_, Self::Item) => true,
            // 5 Bi is node(), and Ai is a KindTest.
            (Self::KindTest(_), Self::KindTest(ast::KindTest::Any)) => true,
            // 6 Bi is text() and Ai is also text().
            (
                Self::KindTest(ast::KindTest::Text),
                Self::KindTest(ast::KindTest::Text),
            ) => true,
            // 7 Bi is comment() and Ai is also comment().
            (
                Self::KindTest(ast::KindTest::Comment),
                Self::KindTest(ast::KindTest::Comment),
            ) => true,
            // 8 Bi is namespace-node() and Ai is also namespace-node().
            (
                Self::KindTest(ast::KindTest::NamespaceNode),
                Self::KindTest(ast::KindTest::NamespaceNode),
            ) => true,
            // 9 Bi is processing-instruction() and Ai is either
            // processing-instruction() or processing-instruction(N) for any
            // name N.
            (
                Self::KindTest(ast::KindTest::PI(_)),
                Self::KindTest(ast::KindTest::PI(None)),
            ) => true,
            // 10 Bi is processing-instruction(Bn), and Ai is also
            // processing-instruction(Bn)
            (
                Self::KindTest(ast::KindTest::PI(Some(a))),
                Self::KindTest(ast::KindTest::PI(Some(b))),
            ) => a == b,
            // 11 Bi is document-node() and Ai is either document-node() or
            // document-node(E) for any ElementTest E.
            (
                Self::KindTest(ast::KindTest::Document(_)),
                Self::KindTest(ast::KindTest::Document(None)),
            ) => true,
            // 12 Bi is document-node(Be) and Ai is document-node(Ae), and
            // subtype-itemtype(Ae, Be).
            (
                Self::KindTest(ast::KindTest::Document(Some(a))),
                Self::KindTest(ast::KindTest::Document(Some(b))),
            ) => a.subtype(b),
            // 13 Bi is either element() or element(*), and Ai is an ElementTest.
            (
                Self::KindTest(ast::KindTest::Element(_)),
                Self::KindTest(ast::KindTest::Element(None)),
            ) => true,
            // 14-18 element comparisons are factored out
            (
                Self::KindTest(ast::KindTest::Element(Some(a))),
                Self::KindTest(ast::KindTest::Element(Some(b))),
            ) => a.subtype(b),
            // 19 Bi is schema-element(Bn), Ai is schema-element(An), and every
            // element declaration that is an actual member of the substitution
            // group of An is also an actual member of the substitution group
            // of Bn.
            // TODO, dummy implementation
            (
                Self::KindTest(ast::KindTest::SchemaElement(_)),
                Self::KindTest(ast::KindTest::SchemaElement(_)),
            ) => false,
            // 20 Bi is either attribute() or attribute(*), and Ai is an
            // AttributeTest.
            (
                Self::KindTest(ast::KindTest::Attribute(_)),
                Self::KindTest(ast::KindTest::Attribute(None)),
            ) => true,
            // 21-23 attribute comparisons are factored out
            (
                Self::KindTest(ast::KindTest::Attribute(Some(a))),
                Self::KindTest(ast::KindTest::Attribute(Some(b))),
            ) => a.subtype(b),
            // 24 Bi is schema-attribute(Bn), the expanded QName of An equals
            // the expanded QName of Bn, and Ai is schema-attribute(An).
            // TODO, dummy implementation
            (
                Self::KindTest(ast::KindTest::SchemaAttribute(_)),
                Self::KindTest(ast::KindTest::SchemaAttribute(_)),
            ) => false,
            // 25 Bi is function(*), Ai is a FunctionTest.
            (
                Self::FunctionTest(_),
                Self::FunctionTest(ast::FunctionTest::AnyFunctionTest),
            ) => true,
            // 26 function comparison factored out
            (
                Self::FunctionTest(ast::FunctionTest::TypedFunctionTest(a)),
                Self::FunctionTest(ast::FunctionTest::TypedFunctionTest(b)),
            ) => a.subtype(b),
            // 27 Ai is map(K, V), for any K and V and Bi is map(*).
            (Self::MapTest(_), Self::MapTest(ast::MapTest::AnyMapTest)) => true,
            // 28 Ai is map(Ka, Va) and Bi is map(Kb, Vb), where
            // subtype-itemtype(Ka, Kb) and subtype(Va, Vb).
            (
                Self::MapTest(ast::MapTest::TypedMapTest(a_typed_map_test)),
                Self::MapTest(ast::MapTest::TypedMapTest(b_typed_map_test)),
            ) => a_typed_map_test.as_ref().subtype(b_typed_map_test.as_ref()),
            // 29 Ai is map(*) (or, because of the transitivity rules, any
            // other map type), and Bi is function(*).
            (
                Self::MapTest(_),
                Self::FunctionTest(ast::FunctionTest::AnyFunctionTest),
            ) => true,

            // 30 Ai is map(*) (or, because of the transitivity rules, any
            // other map type), and Bi is function(xs:anyAtomicType) as
            // item()*.
            (
                Self::MapTest(_),
                Self::FunctionTest(ast::FunctionTest::TypedFunctionTest(
                    typed_function_test,
                )),
            ) if typed_function_test.as_ref() == &map_function_test() => true,
            // 31 Ai is array(X) and Bi is array(*).
            (
                Self::ArrayTest(_),
                Self::ArrayTest(ast::ArrayTest::AnyArrayTest),
            ) => true,
            // 32 Ai is array(X) and Bi is array(Y), and subtype(X, Y) is true.
            (
                Self::ArrayTest(ast::ArrayTest::TypedArrayTest(a_typed_array_test)),
                Self::ArrayTest(ast::ArrayTest::TypedArrayTest(b_typed_array_test)),
            ) if a_typed_array_test
                .as_ref()
                .subtype(b_typed_array_test.as_ref()) =>
            {
                true
            }
            // 33 Ai is array(*) (or, because of the transitivity rules, any
            // other array type) and Bi is function(*).
            (
                Self::ArrayTest(_),
                Self::FunctionTest(ast::FunctionTest::AnyFunctionTest),
            ) => true,

            // 34 Ai is array(*) (or, because of the transitivity rules, any
            // other array type) and Bi is function(xs:integer) as item()*.
            (
                Self::ArrayTest(_),
                Self::FunctionTest(ast::FunctionTest::TypedFunctionTest(
                    typed_function_test,
                )),
            ) if typed_function_test.as_ref() == &array_function_test() => true,
            // 35 Ai is map(K, V), and Bi is function(xs:anyAtomicType) as V?.
            // has to be above 30 to match at all
            (
                Self::MapTest(ast::MapTest::TypedMapTest(typed_map_test)),
                Self::FunctionTest(ast::FunctionTest::TypedFunctionTest(
                    typed_function_test,
                )),
            ) => {
                typed_function_test.parameter_types.len() == 1
                    && typed_function_test.parameter_types[0]
                        == ast::SequenceType::Item(ast::Item {
                            item_type: Self::AtomicOrUnionType(
                                xee_schema_type::Xs::AnyAtomicType,
                            ),
                            occurrence: ast::Occurrence::One,
                        })
                    && is_type_question_mark(
                        &typed_map_test.value_type,
                        &typed_function_test.return_type,
                    )
            }
            // 36 Ai is array(X) and Bi is function(xs:integer) as X.
            (
                Self::ArrayTest(ast::ArrayTest::TypedArrayTest(a_sequence_type)),
                Self::FunctionTest(ast::FunctionTest::TypedFunctionTest(
                    typed_function_test,
                )),
            ) => {
                typed_function_test.parameter_types.len() == 1
                    && typed_function_test.parameter_types[0]
                        == ast::SequenceType::Item(ast::Item {
                            item_type: Self::AtomicOrUnionType(
                                xee_schema_type::Xs::Integer,
                            ),
                            occurrence: ast::Occurrence::One,
                        })
                    && a_sequence_type.item_type == typed_function_test.return_type
            }
            _ => false,
        }
    }
}

// V? occurs in places in the spec. This isn't very well defined, but
// I've analyzed it here: https://github.com/w3c/qt3tests/issues/28
fn is_type_question_mark(a: &ast::SequenceType, b: &ast::SequenceType) -> bool {
    match (a, b) {
        (ast::SequenceType::Empty, ast::SequenceType::Empty) => true,
        (ast::SequenceType::Item(a_item), ast::SequenceType::Item(b_item)) => {
            if a_item != b_item {
                return false;
            }
            match &b_item.occurrence {
                ast::Occurrence::One => false,
                ast::Occurrence::NonEmpty => false,
                ast::Occurrence::Option => true,
                ast::Occurrence::Many => true,
            }
        }
        _ => false,
    }
}

fn map_function_test() -> ast::TypedFunctionTest {
    map_function_test_with_return_type(&ast::SequenceType::Item(ast::Item {
        item_type: ast::ItemType::Item,
        occurrence: ast::Occurrence::Many,
    }))
}

fn map_function_test_with_return_type(return_type: &ast::SequenceType) -> ast::TypedFunctionTest {
    ast::TypedFunctionTest {
        parameter_types: vec![ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::AtomicOrUnionType(xee_schema_type::Xs::AnyAtomicType),
            occurrence: ast::Occurrence::One,
        })],
        return_type: return_type.clone(),
    }
}

fn array_function_test() -> ast::TypedFunctionTest {
    ast::TypedFunctionTest {
        parameter_types: vec![ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::AtomicOrUnionType(xee_schema_type::Xs::Integer),
            occurrence: ast::Occurrence::One,
        })],
        return_type: ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::Item,
            occurrence: ast::Occurrence::Many,
        }),
    }
}

impl TypeInfo for ast::DocumentTest {
    fn subtype(&self, other: &Self) -> bool {
        match (self, other) {
            // duplicate of 13 Bi is either element() or element(*), and Ai is an ElementTest.
            (Self::Element(..), Self::Element(None)) => true,
            (
                Self::Element(Some(a_element_or_attribute_test)),
                Self::Element(Some(b_element_or_attribute_test)),
            ) => a_element_or_attribute_test.subtype(b_element_or_attribute_test),
            // TODO: schema element test
            _ => false,
        }
    }
}

impl TypeInfo for ast::ElementOrAttributeTest {
    fn subtype(&self, other: &Self) -> bool {
        match (self, other) {
            // 14 Bi is either element(Bn) or element(Bn, xs:anyType?), the
            // expanded QName of An equals the expanded QName of Bn, and Ai
            // is either element(An) or element(An, T) or element(An, T?)
            // for any type T.
            (
                Self {
                    name_or_wildcard: ast::NameOrWildcard::Name(a_name),
                    ..
                },
                Self {
                    name_or_wildcard: ast::NameOrWildcard::Name(b_name),
                    type_name:
                        None
                        | Some(ast::TypeName {
                            name: xee_schema_type::Xs::AnyType,
                            can_be_nilled: true,
                        }),
                },
            ) => a_name == b_name,
            // 15 Bi is element(Bn, Bt), the expanded QName of An equals the
            // expanded QName of Bn, Ai is element(An, At), and
            // derives-from(At, Bt) returns true.
            (
                Self {
                    name_or_wildcard: ast::NameOrWildcard::Name(a_name),
                    type_name:
                        Some(ast::TypeName {
                            name: a_type_name,
                            can_be_nilled: false,
                        }),
                },
                Self {
                    name_or_wildcard: ast::NameOrWildcard::Name(b_name),
                    type_name:
                        Some(ast::TypeName {
                            name: b_type_name,
                            can_be_nilled: false,
                        }),
                },
            ) => a_name == b_name && a_type_name.derives_from(*b_type_name),
            // 16 Bi is element(Bn, Bt?), the expanded QName of An equals the
            // expanded QName of Bn, Ai is either element(An, At) or
            // element(An, At?), and derives-from(At, Bt) returns true.
            (
                Self {
                    name_or_wildcard: ast::NameOrWildcard::Name(a_name),
                    type_name:
                        Some(ast::TypeName {
                            name: a_type_name, ..
                        }),
                },
                Self {
                    name_or_wildcard: ast::NameOrWildcard::Name(b_name),
                    type_name:
                        Some(ast::TypeName {
                            name: b_type_name,
                            can_be_nilled: true,
                        }),
                },
            ) => a_name == b_name && a_type_name.derives_from(*b_type_name),
            // 17 Bi is element(*, Bt), Ai is either element(*, At) or
            // element(N, At) for any name N, and derives-from(At, Bt) returns
            // true.
            (
                Self {
                    type_name:
                        Some(ast::TypeName {
                            name: a_type_name,
                            can_be_nilled: false,
                        }),
                    ..
                },
                Self {
                    name_or_wildcard: ast::NameOrWildcard::Wildcard,
                    type_name:
                        Some(ast::TypeName {
                            name: b_type_name,
                            can_be_nilled: false,
                        }),
                },
            ) => a_type_name.derives_from(*b_type_name),
            // 18 Bi is element(*, Bt?), Ai is either element(*, At),
            // element(*, At?), element(N, At), or element(N, At?) for any name
            // N, and derives-from(At, Bt) returns true.
            (
                Self {
                    type_name:
                        Some(ast::TypeName {
                            name: a_type_name, ..
                        }),
                    ..
                },
                Self {
                    name_or_wildcard: ast::NameOrWildcard::Wildcard,
                    type_name:
                        Some(ast::TypeName {
                            name: b_type_name,
                            can_be_nilled: true,
                        }),
                },
            ) => a_type_name.derives_from(*b_type_name),
            _ => false,
        }
    }
}

impl TypeInfo for ast::TypedFunctionTest {
    fn subtype(&self, other: &Self) -> bool {
        // 26 Bi is function(Ba_1, Ba_2, ... Ba_N) as Br, Ai is function(Aa_1,
        // Aa_2, ... Aa_M) as Ar, where N (arity of Bi) equals M (arity of Ai);
        // subtype(Ar, Br); and for values of I between 1 and N, subtype(Ba_I,
        // Aa_I).
        // That is, the arguments are contravariant and the return type is covariant
        if self.parameter_types.len() != other.parameter_types.len() {
            return false;
        }

        // covariant
        if !self.return_type.subtype(&other.return_type) {
            return false;
        }

        for (a, b) in self
            .parameter_types
            .iter()
            .zip(other.parameter_types.iter())
        {
            // contravariant
            if !b.subtype(a) {
                return false;
            }
        }
        true
    }
}

impl TypeInfo for ast::TypedMapTest {
    fn subtype(&self, other: &Self) -> bool {
        self.key_type.derives_from(other.key_type) && self.value_type.subtype(&other.value_type)
    }
}

impl TypeInfo for ast::TypedArrayTest {
    fn subtype(&self, other: &Self) -> bool {
        self.item_type.subtype(&other.item_type)
    }
}
