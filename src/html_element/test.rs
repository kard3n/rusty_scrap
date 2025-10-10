#[cfg(test)]
mod tests {
    use crate::html_element::{extract_attributes, extract_tag, Child, Element, TerminalValue};
    use std::fs;
    use std::ops::Deref;

    #[test]
    fn test_extract_attributes() {
        let source: &str = "a=1 val=\"test\" >";

        let mut iter = source.char_indices();

        let result = extract_attributes(source, &mut iter);
        assert_eq!(result.len(), 2);

        assert!(result[0].name == "a");

        match &result[0].value {
            TerminalValue::Int(s) => {
                assert!(s.deref().eq(&1));
            }
            other => panic!("{:?}", other),
        }
        assert!(result[1].name == "val");
        match &result[1].value {
            TerminalValue::Str(s) => {
                assert_eq!(s.deref(), "test");
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn test_recognizes_self_closing_tag() {
        let source: &str = "<img>";

        let result = Element::from_string(&source);

        match result {
            Element::Root(elems) => match &elems[0] {
                Element::Named(e) => {
                    assert_eq!(e.name, "img");
                }
                _ => panic!("{:?}", &elems[0]),
            },
            _ => panic!("Should be root element!"),
        }
    }

    #[test]
    fn test_recognizes_closing_tag() {
        let source: &str = "<div></div>";

        let result = Element::from_string(&source);

        match result {
            Element::Root(elems) => match &elems[0] {
                Element::Named(e) => {
                    assert_eq!(e.name, "div");
                }
                _ => panic!("{:?}", &elems[0]),
            },
            _ => panic!("Should be root element!"),
        }
    }

    #[test]
    fn test_extract_tag() {
        let source_one: &str = "img>";
        let source_two: &str = "p></p>";

        let mut iter_one = source_one.char_indices();
        let mut iter_two = source_two.char_indices();
        iter_one.next();
        iter_two.next();

        let result_one = extract_tag(source_one, &mut iter_one, true);
        let result_two = extract_tag(source_two, &mut iter_two, true);

        assert!(result_one.is_ok());
        assert!(result_two.is_ok());

        assert_eq!(result_one.unwrap().tag, "img");
        assert_eq!(result_two.unwrap().tag, "p");
    }

    #[test]
    fn test_tag_single_letter() {
        let source: &str = "<p></p>";

        let result = Element::from_string(&source);

        match &result {
            Element::Root(elems) => {
                match &elems[0] { Element::Named(v) => {
                    assert_eq!(v.name, "p");
                }
                    _ => panic!("Expected named element"),
                } }
            _ => {
                panic!("Should be root element!");
            }
        }
        
    }

    #[test]
    #[should_panic]
    fn test_recognizes_missing_closing_tag() {
        let source: &str = "<div><tb></tb>";

        Element::from_string(&source);
    }

    #[test]
    fn finds_attributes() {
        let source: &str = "<img src=\"test\" n= 9 val = 87.0>";

        let result = Element::from_string(&source);

        assert!(matches!(result, Element::Root(..)));
        
        match &result{
            Element::Root(elems) => {
                match &elems[0] {
                    Element::Named(v) => {
                        assert!(v.name == "img");
                        assert_eq!(v.attributes.len(), 3);
                        assert_eq!(v.attributes[0].name, "src");
                        assert_eq!(v.attributes[1].name, "n");
                        assert_eq!(v.attributes[2].name, "val");

                        match &v.attributes[0].value {
                            &TerminalValue::Str("test") => {}
                            other => {
                                panic!("Unexpected value {:?}", other)
                            }
                        }

                        match &v.attributes[1].value {
                            &TerminalValue::Int(9) => {}
                            other => {
                                panic!("Unexpected value {:?}", other)
                            }
                        }

                        match &v.attributes[2].value {
                            &TerminalValue::Float(87.0) => {}
                            other => {
                                panic!("Unexpected value {:?}", other)
                            }
                        }
                    }
                    _ => {
                        panic!("Element was named, detected other.")
                    }
                }
            }
            _ => panic!("Should be root element!"),
        }
    }

    #[test]
    fn finds_text_children() {
        let source: &str = "<div val = 87.0>Hello There!</div>";
        let result = Element::from_string(&source);

        match &result{
            Element::Root(elems) => {
                match &elems[0] {
                    Element::Named(v) => {
                        assert_eq!(v.name, "div");

                        match &v.attributes[0].value {
                            TerminalValue::Float(87.0) => {}
                            other => {
                                panic!("{:?}", other)
                            }
                        }

                        match &v.child {
                            Child::Nodes(v) => {
                                assert_eq!(v.len(), 1);

                                match &v[0] {
                                    Element::Text(t) => {
                                        assert_eq!(t.deref(), "Hello There!");
                                    }
                                    other => panic!("{:?}", other),
                                }
                            }
                            other => {
                                panic!("Unexpected child type: {:?}", other)
                            }
                        }
                    }
                    other => panic!("{:?}", other),
                }
            }
            _ => panic!("Should be root element!"),
        }
        
    }

    #[test]
    fn test_ignore_escaped_opening() {
        let source_one: &str = "<div>To end a div, use \\</t>!</div>";
        let source_two: &str = "<div>\\</t></div>";

        let expected_inner_one = "To end a div, use \\</t>!";
        let expected_inner_two = "\\</t>";

        let result_one = Element::from_string(&source_one);
        let result_two = Element::from_string(&source_two);

        match &result_one {
            Element::Root(elems) => {
                compare_inner_text_element(&elems[0], expected_inner_one, "div");
            }
            _ => panic!("Expected root element!"),
        }

        match &result_two {
            Element::Root(elems) => {
                compare_inner_text_element(&elems[0], expected_inner_two, "div");
            }
            _ => panic!("Expected root element!"),
        }
    }

    fn compare_inner_text_element(result: &Element, expected_inner: &str, expected_name: &str) {
        match result {
            Element::Named(n) => {
                assert_eq!(n.name, expected_name);
                match &n.child {
                    Child::Nodes(children) => {
                        assert_eq!(children.len(), 1);
                        match &children[0] {
                            Element::Text(t) => {
                                assert_eq!(t.deref(), expected_inner);
                            }
                            _ => {
                                panic!("Unexpected named child");
                            }
                        }
                    }
                    _ => {
                        panic! {"Expected nodes, got None"}
                    }
                }
            }
            _ => {
                panic!("{:?}", result)
            }
        }
    }

    #[test]
    fn test_childless_element() {
        let source_one: &str = "<script>This script \\<\t> \\<t> contains a lot</script>";
        let expected = "This script \\<\t> \\<t> contains a lot";
        let result = Element::from_string(&source_one);

        match &result {
            Element::Root(elems) => {
                match &elems[0] {
                    Element::Named(n) => {
                        assert_eq!(n.name, "script");
                        match n.child {
                            Child::Text(content) => {
                                assert_eq!(content.deref(), expected);
                            }
                            _ => {
                                panic! {"Expected nodes, got None"}
                            }
                        }
                    }
                    _ => {
                        panic!("{:?}", result)
                    }
                }
            }
            _ => panic!("Expected root element!"),
        }
    }

    #[test]
    fn test_parse_simple_html() {
        let text = fs::read_to_string("res/test/simple.html").expect("Couldn't read file");

        let result = Element::from_string(&text);

        match &result {
            Element::Root(children) => {
                for child in children {
                    println!("{:?}", child);
                }
                assert_eq!(children.len(), 3);
            }
            _ => {
                panic!("{:?}", result)
            }
        }
    }

    #[test]
    fn test_parse_complex_html() {
        let text = fs::read_to_string("res/test/complex.html").expect("Couldn't read file");

        Element::from_string(&text);
    }
}
