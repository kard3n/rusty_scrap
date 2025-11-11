#[cfg(test)]
mod tests {
    use std::any::Any;
    use crate::mutable_html_element::parser::*;
    use crate::mutable_html_element::*;
    use std::fs;
    use std::ops::Deref;

    #[test]
    fn test_extract_attributes() {
        let source: &str = "a=1 val=\"test\" >";

        let mut iter = source.char_indices();

        let result: AttributeExtractionResult = extract_attributes(source, &mut iter);
        assert_eq!(result.attributes.len(), 2);

        assert!(result.attributes[0].name == "a");
        
        // Check search start and end are correct
        assert_eq!(result.search_start, 0);
        assert_eq!(result.search_end, 15);
        

        match &result.attributes[0].value {
            TerminalValue::Int(s) => {
                assert!(s.deref().eq(&1));
            }
            other => panic!("{:?}", other),
        }
        assert!(result.attributes[1].name == "val");
        match &result.attributes[1].value {
            TerminalValue::Str(s) => {
                assert_eq!(s.deref(), "test");
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn test_recognizes_self_closing_tag() {
        let source: &str = "<img>";

        let result = MutableElement::from_string(&source);
        

        match result {
            MutableElement::Root(elems) => match &elems[0] {
                MutableElement::SelfClosing(e) => {
                    assert_eq!(e.name, "img");
                    assert_eq!(e.start, 0);
                    assert_eq!(e.end, 4);
                }
                _ => panic!("Closing tag not found"),
            },
            _ => panic!("Should be root element!"),
        }
    }

    #[test]
    fn test_recognizes_closing_tag() {
        let source: &str = "<div></div> ";

        let result = MutableElement::from_string(&source);

        match result {
            MutableElement::Root(elems) => match &elems[0] {
                MutableElement::Named(e) => {
                    assert_eq!(e.name, "div");
                }
                _ => panic!("Closing tag not found."),
            },
            _ => panic!("Should be root element!"),
        }
    }

    #[test]
    fn test_extract_tag() {
        let source_one: &str = "<img>";
        let source_two: &str = "<p></p>";

        let mut iter_one = source_one.char_indices();
        let mut iter_two = source_two.char_indices();
        iter_one.next();
        iter_two.next();

        let result_one = extract_tag(source_one, &mut iter_one, 0);
        let result_two = extract_tag(source_two, &mut iter_two, 0);

        assert!(result_one.is_ok());
        assert!(result_two.is_ok());
        
        let res_one_inner = result_one.unwrap();
        let res_two_inner = result_two.unwrap();
        
        assert_eq!(res_one_inner.search_start, 0);
        assert_eq!(res_one_inner.search_end, 4);
        
        assert_eq!(res_two_inner.search_start, 0);
        assert_eq!(res_two_inner.search_end, 2);

        assert_eq!(res_one_inner.tag, "img");
        assert_eq!(res_two_inner.tag, "p");
    }

    #[test]
    fn test_tag_single_letter() {
        let source: &str = "<p></p>";

        let result = MutableElement::from_string(&source);

        match &result {
            MutableElement::Root(elems) => match &elems[0] {
                MutableElement::Named(v) => {
                    assert_eq!(v.name, "p");
                    assert_eq!(v.opening_start, 0);
                    assert_eq!(v.opening_end, 2);
                    assert_eq!(v.closing_start, 3);
                    assert_eq!(v.closing_end, 6);
                }
                _ => panic!("Expected named element"),
            },
            _ => {
                panic!("Should be root element!");
            }
        }
    }

    #[test]
    #[should_panic]
    fn test_recognizes_missing_closing_tag() {
        let source: &str = "<div><tb></tb>";

        MutableElement::from_string(&source);
    }

    #[test]
    fn finds_attributes() {
        let source: &str = "<img src=\"test\" n= 9 val = 87.0>";

        let result = MutableElement::from_string(&source);

        assert!(matches!(result, MutableElement::Root(..)));

        match &result {
            MutableElement::Root(elems) => match &elems[0] {
                MutableElement::SelfClosing(v) => {
                    assert!(v.name == "img");
                    assert_eq!(v.attributes.len(), 3);
                    assert_eq!(v.attributes[0].name, "src");
                    assert_eq!(v.attributes[1].name, "n");
                    assert_eq!(v.attributes[2].name, "val");
                    
                    assert_eq!(v.start, 0);
                    assert_eq!(v.end, 31);

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
            },
            _ => panic!("Should be root element!"),
        }
    }

    #[test]
    fn finds_text_children() {
        let source: &str = "<div val = 87.0>Hello There!</div>";
        let result = MutableElement::from_string(&source);

        match &result {
            MutableElement::Root(elems) => match &elems[0] {
                MutableElement::Named(v) => {
                    assert_eq!(v.name, "div");
                    assert_eq!(v.opening_start, 0);
                    assert_eq!(v.opening_end, 15);
                    assert_eq!(v.closing_start, 29);
                    assert_eq!(v.closing_end, 33);

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
                                MutableElement::Text(t) => {
                                    assert_eq!(t.deref().text, "Hello There!");
                                    assert_eq!(t.start, 16);
                                    assert_eq!(t.end, 27);
                                }
                                other => panic!("Expected text, got other"),
                            }
                        }
                        other => {
                            panic!("Expected Nodes, got else.")
                        }
                    }
                }
                other => panic!("Expected named element, got another"),
            },
            _ => panic!("Should be root element!"),
        }
    }

    #[test]
    fn test_ignore_escaped_opening() {
        let source_one: &str = "<div>To end a div, use \\</t>!</div>";
        let source_two: &str = "<div>\\</t></div>";

        let expected_inner_one = "To end a div, use \\</t>!";
        let expected_inner_two = "\\</t>";

        let result_one = MutableElement::from_string(&source_one);
        let result_two = MutableElement::from_string(&source_two);

        match &result_one {
            MutableElement::Root(elems) => {
                compare_inner_text_element(&elems[0], expected_inner_one, "div");
            }
            _ => panic!("Expected root element!"),
        }

        match &result_two {
            MutableElement::Root(elems) => {
                compare_inner_text_element(&elems[0], expected_inner_two, "div");
            }
            _ => panic!("Expected root element!"),
        }
    }

    fn compare_inner_text_element(result: &MutableElement, expected_inner: &str, expected_name: &str) {
        match result {
            MutableElement::Named(n) => {
                assert_eq!(n.name, expected_name);
                match &n.child {
                    Child::Nodes(children) => {
                        assert_eq!(children.len(), 1);
                        match &children[0] {
                            MutableElement::Text(t) => {
                                assert_eq!(t.deref().text, expected_inner);
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
                panic!("Expected named element, got else")
            }
        }
    }

    #[test]
    fn test_comment() {
        let source_one: &str = "<div><!-- This is a simple comment --></div>";
        let expected = " This is a simple comment ";
        let result = MutableElement::from_string(&source_one);

        match &result {
            MutableElement::Root(elems) => match &elems[0] {
                MutableElement::Named(n) => {
                    assert_eq!(n.name, "div");
                    match &n.child {
                        Child::Nodes(nodes) => match &nodes[0] {
                            MutableElement::Comment(comment) => {
                                assert_eq!(comment.text, expected);
                                assert_eq!(comment.start, 5);
                                assert_eq!(comment.end, 37);
                            }
                            _ => {
                                panic!("Expected a comment, got else.");
                            }
                        },
                        _ => {
                            panic! {"Expected nodes, got None"}
                        }
                    }
                }
                _ => {
                    panic!("Expected named element, got else");
                }
            },
            _ => panic!("Expected root element!"),
        }
    }

    #[test]
    fn test_childless_element() {
        let source_one: &str = "<script>This script \\<\t> \\<t> contains a lot</script>";
        let expected = "This script \\<\t> \\<t> contains a lot";
        let result = MutableElement::from_string(&source_one);

        match &result {
            MutableElement::Root(elems) => match &elems[0] {
                MutableElement::Named(n) => {
                    assert_eq!(n.name, "script");
                    match n.child {
                        Child::Text(content) => {
                            assert_eq!(content, expected);
                        }
                        _ => {
                            panic! {"Expected nodes, got None"}
                        }
                    }
                }
                _ => {
                    panic!("Expected named element, got else");
                }
            },
            _ => panic!("Expected root element!"),
        }
    }
    
    #[test]
    fn test_self_closing_element() {
        let source: &str = "<link href=\"main.css\" rel=\"stylesheet\" />";

        let result = MutableElement::from_string(&source);
        
        match &result {
            MutableElement::Root(r) => {
                match &r[0] {
                    MutableElement::SelfClosing(sc) => {
                        assert_eq!(sc.name, "link");
                        assert_eq!(sc.attributes.len(), 2);
                        assert_eq!(sc.start, 0);
                        assert_eq!(sc.end, 40)
                    }
                    _ => {panic! {"Expected self closing element!"}},
                }
            }
            _other => {
                panic!("Expected root element!");
            }
        }
    }

    #[test]
    fn test_parse_simple_html() {
        let text = fs::read_to_string("res/test/simple.html").expect("Couldn't read file");

        let result = MutableElement::from_string(&text);

        match &result {
            MutableElement::Root(children) => {
                // TODo: check that children are the correct ones
                assert_eq!(children.len(), 3);
            }
            _ => {
                panic!("Expected root element!")
            }
        }
    }

    #[test]
    fn test_parse_complex_html() {
        let text = fs::read_to_string("res/test/complex.html").expect("Couldn't read file");

        MutableElement::from_string(&text);
    }
}
