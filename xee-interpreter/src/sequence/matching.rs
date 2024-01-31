// This module matches types for inline functions or type checks
// The convert module is used for checking and converting values for
// external functions declared with xpath_fn

use xee_schema_type::Xs;
use xee_xpath_ast::ast;
use xee_xpath_ast::parse_sequence_type;
use xee_xpath_ast::Namespaces;
use xot::Xot;

use crate::atomic;
use crate::context;
use crate::error;
use crate::function;
use crate::occurrence::Occurrence;
use crate::xml;
use crate::{sequence::Item, sequence::Sequence};

use super::sequence_type::TypeInfo;

impl Sequence {
    /// Check a type for qee-qt assert-type
    pub fn matches_type<'a>(
        &self,
        s: &str,
        xot: &Xot,
        get_signature: &impl Fn(&function::Function) -> &'a function::Signature,
    ) -> error::Result<bool> {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type(s, &namespaces)?;
        if self
            .clone()
            .sequence_type_matching(&sequence_type, xot, get_signature)
            .is_ok()
        {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // sequence type matching for the purposes of instance of
    pub(crate) fn sequence_type_matching<'a>(
        self,
        sequence_type: &ast::SequenceType,
        xot: &Xot,
        get_signature: &impl Fn(&function::Function) -> &'a function::Signature,
    ) -> error::Result<Self> {
        self.sequence_type_matching_convert(
            sequence_type,
            &|sequence, _| {
                let atomized = sequence.atomized(xot);
                let sequence: Sequence = atomized.collect::<error::Result<Vec<_>>>()?.into();
                Ok(sequence)
            },
            &|function_test, item| item.function_type_matching(function_test, &get_signature),
            xot,
        )
    }

    // sequence type matching, including function conversion rules
    pub(crate) fn sequence_type_matching_function_conversion<'a>(
        self,
        sequence_type: &ast::SequenceType,
        context: &'a context::DynamicContext,
        get_signature: &impl Fn(&function::Function) -> &'a function::Signature,
    ) -> error::Result<Self> {
        self.sequence_type_matching_convert(
            sequence_type,
            &|sequence, xs| Self::convert_atomic(sequence, xs, context),
            &|function_test, item| item.function_arity_matching(function_test, &get_signature),
            context.xot(),
        )
    }

    fn convert_atomic(
        sequence: &Sequence,
        xs: Xs,
        context: &context::DynamicContext,
    ) -> error::Result<Sequence> {
        let atomized = sequence.atomized(context.xot());
        let mut items = Vec::new();
        for atom in atomized {
            let atom = atom?;
            let atom = if matches!(atom, atomic::Atomic::Untyped(_)) {
                atom.cast_to_schema_type(xs, context)?
            } else {
                atom
            };
            let atom = atom.type_promote(xs)?;
            let item = Item::from(atom);
            items.push(item);
        }
        Ok(Sequence::from(items))
    }

    fn sequence_type_matching_convert(
        self,
        t: &ast::SequenceType,
        convert_atomic: &impl Fn(&Sequence, Xs) -> error::Result<Sequence>,
        check_function: &impl Fn(&ast::FunctionTest, &Item) -> error::Result<()>,
        xot: &Xot,
    ) -> error::Result<Self> {
        match t {
            ast::SequenceType::Empty => {
                if self.is_empty() {
                    Ok(self)
                } else {
                    Err(error::Error::XPTY0004)
                }
            }
            ast::SequenceType::Item(occurrence_item) => {
                self.occurrence_item_matching(occurrence_item, convert_atomic, check_function, xot)
            }
        }
    }

    fn occurrence_item_matching(
        self,
        occurrence_item: &ast::Item,
        convert_atomic: &impl Fn(&Sequence, Xs) -> error::Result<Sequence>,
        check_function: &impl Fn(&ast::FunctionTest, &Item) -> error::Result<()>,
        xot: &Xot,
    ) -> error::Result<Self> {
        let sequence = match &occurrence_item.item_type {
            ast::ItemType::AtomicOrUnionType(xs) => convert_atomic(&self, *xs)?,
            _ => self,
        };
        match occurrence_item.occurrence {
            ast::Occurrence::One => {
                let one = sequence.items().one()?;
                one.item_type_matching(
                    &occurrence_item.item_type,
                    convert_atomic,
                    check_function,
                    xot,
                )?;
                Ok(sequence)
            }
            ast::Occurrence::Option => {
                let option = sequence.items().option()?;
                if let Some(item) = option {
                    item.item_type_matching(
                        &occurrence_item.item_type,
                        convert_atomic,
                        check_function,
                        xot,
                    )?;
                    Ok(sequence)
                } else {
                    Ok(sequence)
                }
            }
            ast::Occurrence::Many => {
                for item in sequence.items() {
                    item?.item_type_matching(
                        &occurrence_item.item_type,
                        convert_atomic,
                        check_function,
                        xot,
                    )?;
                }
                Ok(sequence)
            }
            ast::Occurrence::NonEmpty => {
                if sequence.is_empty() {
                    return Err(error::Error::XPTY0004);
                }
                for item in sequence.items() {
                    item?.item_type_matching(
                        &occurrence_item.item_type,
                        convert_atomic,
                        check_function,
                        xot,
                    )?;
                }
                Ok(sequence)
            }
        }
    }
}

impl Item {
    fn item_type_matching(
        &self,
        item_type: &ast::ItemType,
        convert_atomic: &impl Fn(&Sequence, Xs) -> error::Result<Sequence>,
        check_function: &impl Fn(&ast::FunctionTest, &Item) -> error::Result<()>,
        xot: &Xot,
    ) -> error::Result<()> {
        match item_type {
            ast::ItemType::Item => Ok(()),
            ast::ItemType::AtomicOrUnionType(xs) => self.to_atomic()?.atomic_type_matching(*xs),
            ast::ItemType::KindTest(kind_test) => self.kind_test_matching(kind_test, xot),
            ast::ItemType::FunctionTest(function_test) => check_function(function_test, self),
            ast::ItemType::MapTest(map_test) => match map_test {
                ast::MapTest::AnyMapTest => {
                    if self.is_map() {
                        Ok(())
                    } else {
                        Err(error::Error::XPTY0004)
                    }
                }
                ast::MapTest::TypedMapTest(typed_map_test) => {
                    let map = self.to_map()?;
                    for (_, (key, value)) in map.0.iter() {
                        key.atomic_type_matching(typed_map_test.key_type)?;
                        value.clone().sequence_type_matching_convert(
                            &typed_map_test.value_type,
                            convert_atomic,
                            check_function,
                            xot,
                        )?;
                    }
                    Ok(())
                }
            },
            ast::ItemType::ArrayTest(array_test) => match array_test {
                ast::ArrayTest::AnyArrayTest => {
                    if self.is_array() {
                        Ok(())
                    } else {
                        Err(error::Error::XPTY0004)
                    }
                }
                ast::ArrayTest::TypedArrayTest(typed_array_test) => {
                    let array = self.to_array()?;
                    for sequence in array.iter() {
                        sequence.clone().sequence_type_matching_convert(
                            &typed_array_test.item_type,
                            convert_atomic,
                            check_function,
                            xot,
                        )?;
                    }

                    Ok(())
                }
            },
        }
    }

    fn kind_test_matching(&self, kind_test: &ast::KindTest, xot: &Xot) -> error::Result<()> {
        match self {
            Item::Node(node) => {
                if xml::kind_test(kind_test, xot, *node) {
                    Ok(())
                } else {
                    Err(error::Error::XPTY0004)
                }
            }
            Item::Atomic(_) => Err(error::Error::XPTY0004),
            Item::Function(_) => Err(error::Error::XPTY0004),
        }
    }

    fn function_arity_matching<'a>(
        &self,
        function_test: &ast::FunctionTest,
        get_signature: &impl Fn(&function::Function) -> &'a function::Signature,
    ) -> error::Result<()> {
        match function_test {
            ast::FunctionTest::AnyFunctionTest => {
                self.to_function()?;
                Ok(())
            }
            ast::FunctionTest::TypedFunctionTest(typed_function_test) => {
                let function = self.to_function()?;
                let signature = get_signature(&function);
                if signature.arity() == typed_function_test.parameter_types.len() {
                    Ok(())
                } else {
                    Err(error::Error::XPTY0004)
                }
            }
        }
    }

    fn function_type_matching<'a>(
        &self,
        function_test: &ast::FunctionTest,
        get_signature: &impl Fn(&function::Function) -> &'a function::Signature,
    ) -> error::Result<()> {
        match function_test {
            ast::FunctionTest::AnyFunctionTest => {
                self.to_function()?;
                Ok(())
            }
            ast::FunctionTest::TypedFunctionTest(typed_function_test) => {
                let function = self.to_function()?;
                let signature = get_signature(&function);
                if signature.arity() != typed_function_test.parameter_types.len() {
                    return Err(error::Error::XPTY0004);
                }
                if Self::function_type_matching_helper(typed_function_test, signature) {
                    Ok(())
                } else {
                    Err(error::Error::XPTY0004)
                }
            }
        }
    }

    fn function_type_matching_helper(
        function_test: &ast::TypedFunctionTest,
        signature: &function::Signature,
    ) -> bool {
        let default_sequence_type = Self::default_sequence_type();
        let function_return_type = signature
            .return_type
            .as_ref()
            .unwrap_or(&default_sequence_type);
        // return type is covariant
        if !function_return_type.subtype(&function_test.return_type) {
            return false;
        }

        for (function_parameter, test_parameter) in signature
            .parameter_types
            .iter()
            .zip(&function_test.parameter_types)
        {
            let function_parameter = function_parameter
                .as_ref()
                .unwrap_or(&default_sequence_type);
            // parameter is contravariant
            if !test_parameter.subtype(function_parameter) {
                return false;
            }
        }

        true
    }

    fn default_sequence_type() -> ast::SequenceType {
        ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::Item,
            occurrence: ast::Occurrence::Many,
        })
    }
}

impl atomic::Atomic {
    fn atomic_type_matching(&self, xs: Xs) -> error::Result<()> {
        let schema_type = self.schema_type();
        if schema_type.derives_from(xs) || schema_type.matches(xs) {
            Ok(())
        } else {
            Err(error::Error::XPTY0004)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use ibig::ibig;

    use xee_name::Namespaces;
    use xee_xpath_ast::parse_sequence_type;

    use crate::xml;

    #[test]
    fn test_one_integer() {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type("xs:integer", &namespaces).unwrap();

        let right_sequence = Sequence::from(vec![Item::from(ibig!(1))]);
        let wrong_amount_sequence =
            Sequence::from(vec![Item::from(ibig!(1)), Item::from(ibig!(2))]);
        let wrong_type_sequence = Sequence::from(vec![Item::from(atomic::Atomic::from(false))]);
        let xot = Xot::new();

        let right_result = right_sequence.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(&right_result.unwrap(), &right_sequence);

        let wrong_amount_result =
            wrong_amount_sequence.sequence_type_matching(&sequence_type, &xot, &|_| unreachable!());
        assert_eq!(wrong_amount_result, Err(error::Error::XPTY0004));
        let wrong_type_result =
            wrong_type_sequence.sequence_type_matching(&sequence_type, &xot, &|_| unreachable!());
        assert_eq!(wrong_type_result, Err(error::Error::XPTY0004));
    }

    #[test]
    fn test_one_long_matches_integer() {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type("xs:integer", &namespaces).unwrap();

        let right_sequence = Sequence::from(vec![Item::from(1i64)]);
        let wrong_amount_sequence = Sequence::from(vec![Item::from(1i64), Item::from(1i64)]);
        let wrong_type_sequence = Sequence::from(vec![Item::from(atomic::Atomic::from(false))]);
        let xot = Xot::new();

        let right_result = right_sequence.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_result, Ok(right_sequence));
        let wrong_amount_result =
            wrong_amount_sequence.sequence_type_matching(&sequence_type, &xot, &|_| unreachable!());
        assert_eq!(wrong_amount_result, Err(error::Error::XPTY0004));
        let wrong_type_result =
            wrong_type_sequence.sequence_type_matching(&sequence_type, &xot, &|_| unreachable!());
        assert_eq!(wrong_type_result, Err(error::Error::XPTY0004));
    }

    #[test]
    fn test_one_any_atomic() {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type("xs:anyAtomicType", &namespaces).unwrap();

        let right_sequence = Sequence::from(vec![Item::from(atomic::Atomic::from(1i64))]);
        let wrong_amount_sequence =
            Sequence::from(vec![Item::from(ibig!(1)), Item::from(ibig!(2))]);
        let right_type_sequence2 = Sequence::from(vec![Item::from(atomic::Atomic::from(false))]);
        let xot = Xot::new();

        let right_result = right_sequence.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_result, Ok(right_sequence));
        let wrong_amount_result =
            wrong_amount_sequence.sequence_type_matching(&sequence_type, &xot, &|_| unreachable!());
        assert_eq!(wrong_amount_result, Err(error::Error::XPTY0004));
        let right_type_result2 = right_type_sequence2.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_type_result2, Ok(right_type_sequence2));
    }

    #[test]
    fn test_one_item() {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type("item()", &namespaces).unwrap();
        let mut xot = Xot::new();
        let root = xot.parse("<doc/>").unwrap();
        let node = xot.document_element(root).unwrap();
        let node = xml::Node::Xot(node);
        let right_sequence = Sequence::from(vec![Item::from(atomic::Atomic::from(1i64))]);
        let wrong_amount_sequence = Sequence::from(vec![
            Item::from(atomic::Atomic::from(1i64)),
            Item::from(atomic::Atomic::from(2i64)),
        ]);
        let right_type_sequence2 = Sequence::from(vec![Item::from(node)]);

        let right_result = right_sequence.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_result, Ok(right_sequence));

        let wrong_amount_result =
            wrong_amount_sequence.sequence_type_matching(&sequence_type, &xot, &|_| unreachable!());
        assert_eq!(wrong_amount_result, Err(error::Error::XPTY0004));
        let right_type_result2 = right_type_sequence2.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_type_result2, Ok(right_type_sequence2));
    }

    #[test]
    fn test_option_integer() {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type("xs:integer?", &namespaces).unwrap();

        let right_sequence = Sequence::from(vec![Item::from(atomic::Atomic::from(ibig!(1)))]);
        let wrong_amount_sequence =
            Sequence::from(vec![Item::from(ibig!(1)), Item::from(ibig!(2))]);
        let right_empty_sequence = Sequence::empty();
        let xot = Xot::new();

        let right_result = right_sequence.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_result, Ok(right_sequence));
        let wrong_amount_result =
            wrong_amount_sequence.sequence_type_matching(&sequence_type, &xot, &|_| unreachable!());
        assert_eq!(wrong_amount_result, Err(error::Error::XPTY0004));
        let right_empty_result = right_empty_sequence.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_empty_result, Ok(right_empty_sequence));
    }

    #[test]
    fn test_many_integer() {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type("xs:integer*", &namespaces).unwrap();

        let right_sequence = Sequence::from(vec![Item::from(atomic::Atomic::from(ibig!(1)))]);
        let right_multi_sequence = Sequence::from(vec![Item::from(ibig!(1)), Item::from(ibig!(2))]);
        let right_empty_sequence = Sequence::empty();
        let xot = Xot::new();

        let right_result = right_sequence.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_result, Ok(right_sequence));

        let right_multi_result = right_multi_sequence.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_multi_result, Ok(right_multi_sequence));

        let right_empty_result = right_empty_sequence.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_empty_result, Ok(right_empty_sequence));
    }

    #[test]
    fn test_many_node() {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type("node()*", &namespaces).unwrap();

        let mut xot = Xot::new();
        let doc = xot.parse(r#"<doc><a attr="Attr">A</a><b/></doc>"#).unwrap();
        let doc = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc).unwrap();
        let b = xot.next_sibling(a).unwrap();
        let text = xot.first_child(a).unwrap();

        let doc = xml::Node::Xot(doc);
        let attr = xml::Node::Attribute(a, xot.name("attr").unwrap());
        let a = xml::Node::Xot(a);
        let b = xml::Node::Xot(b);
        let text = xml::Node::Xot(text);

        let right_sequence = Sequence::from(vec![
            Item::from(doc),
            Item::from(a),
            Item::from(b),
            Item::from(text),
            Item::from(attr),
        ]);

        let wrong_sequence = Sequence::from(vec![Item::from(ibig!(1))]);

        let right_result = right_sequence.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_result, Ok(right_sequence));

        let wrong_result =
            wrong_sequence.sequence_type_matching(&sequence_type, &xot, &|_| unreachable!());
        assert_eq!(wrong_result, Err(error::Error::XPTY0004));
    }

    #[test]
    fn test_many_element() {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type("element()*", &namespaces).unwrap();

        let mut xot = Xot::new();
        let doc = xot.parse(r#"<doc><a attr="Attr">A</a><b/></doc>"#).unwrap();
        let doc = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc).unwrap();
        let b = xot.next_sibling(a).unwrap();
        let text = xot.first_child(a).unwrap();

        let doc = xml::Node::Xot(doc);
        let attr = xml::Node::Attribute(a, xot.name("attr").unwrap());
        let a = xml::Node::Xot(a);
        let b = xml::Node::Xot(b);
        let text = xml::Node::Xot(text);

        let right_sequence = Sequence::from(vec![Item::from(doc), Item::from(a), Item::from(b)]);

        let wrong_sequence_text = Sequence::from(vec![Item::from(text)]);
        let wrong_sequence_attr = Sequence::from(vec![Item::from(attr)]);

        let right_result = right_sequence.clone().sequence_type_matching(
            &sequence_type,
            &xot,
            &|_| unreachable!(),
        );
        assert_eq!(right_result, Ok(right_sequence));

        let wrong_result =
            wrong_sequence_text.sequence_type_matching(&sequence_type, &xot, &|_| unreachable!());
        assert_eq!(wrong_result, Err(error::Error::XPTY0004));
        let wrong_result =
            wrong_sequence_attr.sequence_type_matching(&sequence_type, &xot, &|_| unreachable!());
        assert_eq!(wrong_result, Err(error::Error::XPTY0004));
    }

    #[test]
    fn test_many_atomized_promote() {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type("xs:double*", &namespaces).unwrap();

        // integers count as decimals, so should be promoted to a double
        let right_sequence = Sequence::from(vec![Item::from(ibig!(1)), Item::from(ibig!(2))]);

        let xot = Xot::new();
        let static_context = context::StaticContext::default();
        let dynamic_context = context::DynamicContext::empty(xot, &static_context);

        let right_result = right_sequence.sequence_type_matching_function_conversion(
            &sequence_type,
            &dynamic_context,
            &|_| unreachable!(),
        );
        // atomization has changed the result sequence
        assert_eq!(
            right_result,
            Ok(Sequence::from(vec![
                Item::from(atomic::Atomic::from(1f64)),
                Item::from(atomic::Atomic::from(2f64)),
            ]))
        );
    }

    #[test]
    fn test_many_cast_untyped() {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type("xs:integer*", &namespaces).unwrap();

        let mut xot = Xot::new();
        let doc = xot.parse(r#"<doc><a>1</a><b>2</b></doc>"#).unwrap();
        let doc = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc).unwrap();
        let b = xot.next_sibling(a).unwrap();

        let a = xml::Node::Xot(a);
        let b = xml::Node::Xot(b);

        let right_sequence = Sequence::from(vec![Item::from(a), Item::from(b)]);

        let static_context = context::StaticContext::default();
        let dynamic_context = context::DynamicContext::empty(xot, &static_context);

        let right_result = right_sequence.sequence_type_matching_function_conversion(
            &sequence_type,
            &dynamic_context,
            &|_| unreachable!(),
        );
        // atomization has changed the result sequence
        assert_eq!(
            right_result,
            Ok(Sequence::from(vec![
                Item::from(atomic::Atomic::from(ibig!(1))),
                Item::from(atomic::Atomic::from(ibig!(2))),
            ]))
        );
    }

    #[test]
    fn test_any_function_test() {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type("function(*)", &namespaces).unwrap();
        let function = function::Function::Static {
            static_function_id: function::StaticFunctionId(1),
            closure_vars: vec![],
        };
        let right_sequence = Sequence::from(vec![Item::Function(Rc::new(function))]);

        let signature = function::Signature {
            parameter_types: vec![Some(
                parse_sequence_type("xs:integer", &namespaces).unwrap(),
            )],
            return_type: Some(parse_sequence_type("xs:integer", &namespaces).unwrap()),
        };

        let xot = Xot::new();

        let right_result =
            right_sequence
                .clone()
                .sequence_type_matching(&sequence_type, &xot, &|_| &signature);
        assert_eq!(&right_result.unwrap(), &right_sequence);
    }

    #[test]
    fn test_function_test_same_parameters() {
        let namespaces = Namespaces::default();
        let sequence_type =
            parse_sequence_type("function(xs:integer) as xs:integer", &namespaces).unwrap();
        let function = function::Function::Static {
            static_function_id: function::StaticFunctionId(1),
            closure_vars: vec![],
        };
        let right_sequence = Sequence::from(vec![Item::Function(Rc::new(function))]);

        let signature = function::Signature {
            parameter_types: vec![Some(
                parse_sequence_type("xs:integer", &namespaces).unwrap(),
            )],
            return_type: Some(parse_sequence_type("xs:integer", &namespaces).unwrap()),
        };

        let xot = Xot::new();

        let right_result =
            right_sequence
                .clone()
                .sequence_type_matching(&sequence_type, &xot, &|_| &signature);
        assert_eq!(&right_result.unwrap(), &right_sequence);
    }

    #[test]
    fn test_function_test_derived_parameters() {
        let namespaces = Namespaces::default();
        let sequence_type =
            parse_sequence_type("function(xs:integer) as xs:integer", &namespaces).unwrap();
        let function = function::Function::Static {
            static_function_id: function::StaticFunctionId(1),
            closure_vars: vec![],
        };
        let right_sequence = Sequence::from(vec![Item::Function(Rc::new(function))]);

        let signature = function::Signature {
            parameter_types: vec![Some(
                parse_sequence_type("xs:integer", &namespaces).unwrap(),
            )],
            return_type: Some(parse_sequence_type("xs:integer", &namespaces).unwrap()),
        };

        let xot = Xot::new();

        let right_result =
            right_sequence
                .clone()
                .sequence_type_matching(&sequence_type, &xot, &|_| &signature);
        assert_eq!(&right_result.unwrap(), &right_sequence);
    }

    #[test]
    fn test_function_test_wrong_arity() {
        let namespaces = Namespaces::default();
        let sequence_type =
            parse_sequence_type("function(xs:integer) as xs:integer", &namespaces).unwrap();
        let function = function::Function::Static {
            static_function_id: function::StaticFunctionId(1),
            closure_vars: vec![],
        };
        let wrong_sequence = Sequence::from(vec![Item::Function(Rc::new(function))]);

        let signature = function::Signature {
            parameter_types: vec![
                Some(parse_sequence_type("xs:integer", &namespaces).unwrap()),
                Some(parse_sequence_type("xs:integer", &namespaces).unwrap()),
            ],
            return_type: Some(parse_sequence_type("xs:integer", &namespaces).unwrap()),
        };

        let xot = Xot::new();

        let wrong_result =
            wrong_sequence
                .clone()
                .sequence_type_matching(&sequence_type, &xot, &|_| &signature);
        assert_eq!(wrong_result, Err(error::Error::XPTY0004));
    }
}
