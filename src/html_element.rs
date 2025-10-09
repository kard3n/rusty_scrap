static WHITESPACE_SYMBOLS: [char; 3] = [' ', '\t', '\n'];
static SELF_CLOSING_TAGS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr", "!DOCTYPE"
];
static CHILDLESS_TAGS: [&str; 4] = ["script", "style", "textarea", "title"];

#[derive(Debug)]
pub enum TerminalValue<'a> {
    Int(i32),
    Float(f32),
    Str(&'a str),
}

#[derive(Debug)]
pub enum Element<'a> {
    Named(NamedElement<'a>),
    Text(&'a str),
}

enum IteratorResultElement<'a> {
    Element {
        element: Element<'a>,
        last_char: char,
    },
    ClosingTag(&'a str),
    End,
}

#[derive(Debug)]
pub enum Child<'a> {
    None,                    // Does not have a child
    Nodes(Vec<Element<'a>>), // One or more children
    Text(&'a str),           // Text content, like in <script>
}

#[derive(Debug)]
pub struct Attribute<'a> {
    pub name: &'a str,
    pub value: TerminalValue<'a>,
}

struct TagExtractionResult<'a> {
    tag: &'a str,
    last_char_higher_than: bool,
}

#[derive(Debug)]
pub struct NamedElement<'a> {
    pub name: &'a str,
    pub attributes: Vec<Attribute<'a>>,
    pub child: Child<'a>,
}

impl<'a> NamedElement<'a> {
    fn new(name: &'a str, attributes: Vec<Attribute<'a>>, child: Child<'a>) -> Self {
        NamedElement {
            name,
            attributes,
            child,
        }
    }
}

impl<'a> TagExtractionResult<'a> {
    fn new(tag: &'a str, last_char_higher_than: bool) -> Self {
        Self {
            tag,
            last_char_higher_than,
        }
    }
}

impl<'a> Element<'a> {
    pub fn from_string(source: &'a str) -> Self {
        let mut iter = source.char_indices();
        return match Self::from_iterator(&mut iter, &source, false) {
            IteratorResultElement::Element { element, .. } => element,
            IteratorResultElement::ClosingTag(_) => {
                panic!("Parser encountered closing tag as first entry.")
            }
            _ => {
                panic!("Provided string was empty")
            }
        };
    }

    fn from_iterator(
        iter: &mut impl Iterator<Item = (usize, char)>,
        source: &'a str,
        last_char_lower_than: bool,
    ) -> IteratorResultElement<'a> {
        let mut current = iter.next();
        let mut last_was_escape = false;

        if current.is_none() {
            return IteratorResultElement::End;
        }

        if current.unwrap().1 != '<' && !last_char_lower_than {
            // Text element
            let start_pos: usize = current.unwrap().0;
            while current.is_some() && !(current.unwrap().1 == '<' && !last_was_escape) {
                if current.unwrap().1 == '\\' {
                    last_was_escape = true;
                } else if last_was_escape {
                    last_was_escape = false;
                }
                current = iter.next();
            }

            match current {
                None => IteratorResultElement::Element {
                    element: Element::Text(&source[start_pos..]),
                    last_char: '_',
                },
                Some(v) => IteratorResultElement::Element {
                    element: Element::Text(&source[start_pos..v.0]),
                    last_char: v.1,
                },
            }
        } else {
            // Named element
            // First is a <, go to next
            if !last_char_lower_than {
                current = iter.next();
            }

            if current.unwrap().1 == '/' {
                // Closing tag of normal element

                return IteratorResultElement::ClosingTag(
                    &source[current.unwrap().0..{
                        while current.unwrap().1 != '>' {
                            // Go to closing symbol
                            current = iter.next()
                        }
                        current.unwrap().0
                    }],
                );
            } else {
                // Opening tag of an element
                let element_start: usize = current.unwrap().0;
                let tag_extraction_result = extract_tag(&source, iter, true);
                let name: &str = match &tag_extraction_result {
                    Ok(v) => v.tag,
                    Err(c) => {
                        panic!("{}", c);
                    }
                };

                let attributes: Vec<Attribute<'a>> = match &tag_extraction_result {
                    Ok(v) => {
                        if v.last_char_higher_than {
                            Vec::new()
                        } else {
                            extract_attributes(&source, iter)
                        }
                    }
                    Err(c) => {
                        panic!("{}", c);
                    }
                };
                let mut child_nodes: Vec<Element<'a>> = Vec::new();

                if CHILDLESS_TAGS.contains(&name) {
                    // Childless tag, only need to look for it's closing tag. Ignore everything until it was found

                    current = iter.next(); // Jump > after name
                    let start_pos: usize = current.unwrap().0;

                    let mut pattern = String::with_capacity(name.len() + 3);
                    pattern.push_str("</");
                    pattern.push_str(&name);
                    pattern.push_str(">");
                    let mut pattern_iter = pattern.chars();
                    let mut pattern_current = pattern_iter.next();

                    while current.is_some() && pattern_current.is_some() {
                        if current.unwrap().1 == pattern_current.unwrap() {
                            pattern_current = pattern_iter.next();
                        } else {
                            // Reset
                            pattern_iter = pattern.chars();
                            pattern_current = pattern_iter.next();
                        }
                        current = iter.next();
                    }

                    if pattern_current.is_none() {
                        // Pattern was exhausted: success
                        return IteratorResultElement::Element {
                            element: Element::Named(NamedElement {
                                name,
                                attributes,
                                child: Child::Text(
                                    &source[start_pos..{
                                        if current.is_some() {
                                            current.unwrap().0
                                        } else {
                                            source.len()
                                        }
                                    } - pattern.len()],
                                ),
                            }),
                            last_char: '>',
                        };
                    } else {
                        panic!(
                            "Reached EOF searching end tag for tag {} at pos {}",
                            name,
                            start_pos - name.len() - 1
                        );
                    }
                } else if !SELF_CLOSING_TAGS.contains(&name) {
                    // Not a self-cosing tag, therefore can have children
                    let mut next_child = Self::from_iterator(iter, source, false);
                    let mut last_char_lower_than;
                    while !(matches!(next_child, IteratorResultElement::ClosingTag(_))
                        || matches!(next_child, IteratorResultElement::End))
                    {
                        last_char_lower_than = false;
                        match next_child {
                            IteratorResultElement::Element { element, last_char } => {
                                child_nodes.push(element);
                                last_char_lower_than = last_char == '<';
                            }
                            _ => {}
                        }

                        next_child = Self::from_iterator(iter, &source, last_char_lower_than);
                    }

                    // Check that the element was closed properly
                    match next_child {
                        IteratorResultElement::ClosingTag(n) => {
                            if name != &n[1..] {
                                panic!(
                                    "Name of the closing tag ('{}') does not match name of the opening tag ('{}') at {}.",
                                    &n,
                                    name,
                                    element_start - 1
                                )
                            }
                        }
                        _ => {
                            panic!(
                                "Element '{}' at position {} did not have a closing tag!",
                                name,
                                element_start - 1
                            );
                        }
                    }
                }

                return IteratorResultElement::Element {
                    element: Element::Named(NamedElement {
                        name: name,
                        attributes: attributes,
                        child: if !child_nodes.is_empty() {
                            Child::Nodes(child_nodes)
                        } else {
                            Child::None
                        },
                    }),
                    last_char: '_',
                };
            }
        }
    }
}

impl<'a> Attribute<'a> {
    fn new(name: &'a str, value: TerminalValue<'a>) -> Attribute<'a> {
        Attribute { name, value }
    }

    fn from_string(source: &'a str) -> Self {
        let first_equals_sign: usize = source.find('=').unwrap();
        let value = &source[first_equals_sign + 1..].trim_start().trim_end();

        return if value.starts_with('"') {
            Attribute::new(
                &source[..first_equals_sign].trim_end(),
                TerminalValue::Str(&value[1..value.len() - 1]),
            )
        } else if value.contains('.') {
            Attribute::new(
                &source[..first_equals_sign].trim_end(),
                TerminalValue::Float(value.parse::<f32>().unwrap()),
            )
        } else {
            Attribute::new(
                &source[..first_equals_sign].trim_end(),
                TerminalValue::Int(value.parse::<i32>().unwrap()),
            )
        };
    }
}

/// Extracts HTML attributes from the given string
/// # Arguments
/// * `source` A string slice containing the whole document
/// * `iter` mutable reference to an iterator. Current position must be set to after the tag of the element, and before start of the attributes
fn extract_attributes<'a>(
    source: &'a str,
    iter: &mut impl Iterator<Item = (usize, char)>,
) -> Vec<Attribute<'a>> {
    // Find area containing the attributes

    let mut previous_was_escape: bool = false;
    let mut in_string: bool = false;

    let mut attributes: Vec<Attribute<'a>> = Vec::new();

    let mut current_attr_start: Option<usize> = None;
    let mut after_equal_sign: bool = false;
    let mut before_value_start: bool = true;

    let mut current = iter.next();

    // Jump whitespaces
    while current.is_some() && WHITESPACE_SYMBOLS.contains(&current.unwrap().1) {
        current = iter.next();
    }

    while current.is_some() && !(!in_string && current.unwrap().1 == '>') {
        // Logic for separating attributes
        {
            if current_attr_start.is_none() {
                current_attr_start = Some(current.unwrap().0);
            }

            if current.unwrap().1 == '=' && !after_equal_sign {
                after_equal_sign = true;
            } else if after_equal_sign
                && before_value_start
                && !WHITESPACE_SYMBOLS.contains(&current.unwrap().1)
            {
                before_value_start = false;
            } else if WHITESPACE_SYMBOLS.contains(&current.unwrap().1)
                && !before_value_start
                && !in_string
            {
                attributes.push(Attribute::from_string(
                    &source[current_attr_start.unwrap()..current.unwrap().0],
                ));
                before_value_start = true;
                after_equal_sign = false;
                current_attr_start = None;
            }
        }

        if current.unwrap().1 == '"' && !previous_was_escape {
            in_string = !in_string;
        }
        if current.unwrap().1 == '\\' && !previous_was_escape {
            previous_was_escape = true;
            current = iter.next();
            continue;
        }
        previous_was_escape = false;
        current = iter.next();
    }
    // Add last attribute
    if !before_value_start && !in_string {
        attributes.push(Attribute::from_string(
            &source[current_attr_start.unwrap()..current.unwrap().0],
        ));
    }

    attributes
}

/// Returns a string slice containing only the tag
/// # Arguments
/// * `source` The beginning of an HTML element. Must have a `<` followed by a name. If a closing tag, the `/` will be included in the result
/// * `iter` mutable reference to an iterator. Current position must be set to before the start of the element (`<`)
/// * `at_opening` whether the iterator as already at the first letter of the name. If set to false, will advance to first '<'
/// # Returns
/// A `Result::Ok` containing the tag if successful. Otherwise,a `Result::Err` containing an error code
fn extract_tag<'a>(
    source: &'a str,
    iter: &mut impl Iterator<Item = (usize, char)>,
    at_opening: bool,
) -> Result<TagExtractionResult<'a>, &'a str> {
    let name_start: usize;
    let name_end: usize;
    let mut current: Option<(usize, char)> = iter.next();

    // Find start of name
    // Find opening symbol
    if !at_opening {
        while current.is_some() && current.unwrap().1 != '<' {
            current = iter.next();
        }
    }

    if current.is_some() {
        // Found opening
        // Jump whitespaces
        while current.is_some() && WHITESPACE_SYMBOLS.contains(&current.unwrap().1) {
            current = iter.next();
        }

        // Set start to index if available
        name_start = match current {
            None => {
                return Result::Err("String ended after opening '<'");
            }
            Some(v) => v.0,
        };
    } else {
        return Result::Err("Element did not contain start '<'");
    }

    // Find end of name
    while current.is_some() && (current.unwrap().1.is_alphanumeric() || current.unwrap().1 == '/') {
        current = iter.next();
    }
    name_end = match current {
        None => {
            return Result::Err("Name not completed before string end");
        }
        Some(v) => v.0,
    };

    return Result::Ok(TagExtractionResult::new(
        &source[if at_opening {
            name_start - 1
        } else {
            name_start
        }..name_end],
        current.is_some() && current.unwrap().1 == '>',
    ));
}

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
            Element::Named(e) => {
                assert_eq!(e.name, "img");
            }
            _ => panic!("{:?}", result),
        }
    }

    #[test]
    fn test_recognizes_closing_tag() {
        let source: &str = "<div></div>";

        let result = Element::from_string(&source);

        match result {
            Element::Named(v) => {
                assert_eq!(v.name, "div");
            }
            _ => panic!("Expected named element"),
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

        match result {
            Element::Named(v) => {
                assert_eq!(v.name, "p");
            }
            _ => panic!("Expected named element"),
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

        assert!(matches!(result, Element::Named(..)));

        match result {
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
            Element::Text(_) => {
                panic!("Element was named, detected unnamed.")
            }
        }
    }

    #[test]
    fn finds_text_children() {
        let source: &str = "<div val = 87.0>Hello There!</div>";
        let result = Element::from_string(&source);

        match result {
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

    #[test]
    fn test_ignore_escaped_opening() {
        let source_one: &str = "<div>To end a div, use \\</t>!</div>";
        let source_two: &str = "<div>\\</t></div>";

        let expected_inner_one = "To end a div, use \\</t>!";
        let expected_inner_two = "\\</t>";

        let result_one = Element::from_string(&source_one);
        let result_two = Element::from_string(&source_two);

        compare_inner_text_element(result_one, expected_inner_one, "div");
        compare_inner_text_element(result_two, expected_inner_two, "div");
    }

    fn compare_inner_text_element(result: Element, expected_inner: &str, expected_name: &str) {
        match result {
            Element::Named(n) => {
                assert_eq!(n.name, expected_name);
                match n.child {
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

        match result {
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

    #[test]
    fn test_large() {
        let text = fs::read_to_string("res/test/simple_html.txt").expect("Couldn't read file");

        let result = Element::from_string(&text);
    }
}
