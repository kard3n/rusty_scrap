mod test;

const WHITESPACE_SYMBOLS: [char; 3] = [' ', '\t', '\n'];
const VALID_TAG_CHARS: [char; 64] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '-', '.',
];
const SELF_CLOSING_TAGS: [&str; 13] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];
const CHILDLESS_TAGS: [&str; 4] = ["script", "style", "textarea", "title"];

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
    Comment(&'a str),
    Root(Vec<Element<'a>>),
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

struct PatternSearchResult {
    search_start_pos: usize,
    result: Option<(usize, char)>,
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

impl<'a> Element<'a> {
    pub fn from_string(source: &'a str) -> Self {
        let mut iter = source.char_indices();
        let mut result: Vec<Element<'a>> = Vec::new();

        let mut current_result: IteratorResultElement;
        let mut last_char_lower_than = false;
        let mut stop: bool = false;

        while !stop {
            current_result = Self::from_iterator(&mut iter, &source, last_char_lower_than);
            match current_result {
                IteratorResultElement::Element { element, last_char } => {
                    result.push(element);
                    last_char_lower_than = last_char == '<';
                }
                IteratorResultElement::ClosingTag(..) => {
                    panic!("Encountered closing tag as direct child of the root element");
                }
                IteratorResultElement::End => {
                    stop = true;
                }
            }
        }

        return Self::Root(result);
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

                // Special case: comment
                // Don't need to read attributes, but look for special closing -->
                if name.starts_with("!--") {
                    // Comment
                    let search_result = find_pattern(iter, "-->");

                    match search_result {
                        Result::Ok(result) => {
                            return IteratorResultElement::Element {
                                element: Element::Comment(
                                    &source[element_start + name.len()
                                        ..if result.result.is_some() {
                                            result.result.unwrap().0 - 2
                                        } else {
                                            source.len()
                                        }],
                                ),
                                last_char: '>',
                            };
                        }
                        Result::Err(c) => {
                            panic!("{}", c);
                        }
                    }
                }

                let attributes: Vec<Attribute<'a>> = match &tag_extraction_result {
                    Ok(v) => {
                        if v.last_char_higher_than {
                            println!("1");
                            Vec::new()
                        } else {
                            println!("2");
                            extract_attributes(&source, iter)
                        }
                    }
                    Err(c) => {
                        panic!("{}", c);
                    }
                };
                let mut child_nodes: Vec<Element<'a>> = Vec::new();

                if name.starts_with('!') {
                    // Doctype
                    return IteratorResultElement::Element {
                        element: Element::Named(NamedElement::new(name, attributes, Child::None)),
                        last_char: '_',
                    };
                } else if CHILDLESS_TAGS.contains(&name) {
                    // Childless tag, only need to look for it's closing tag. Ignore everything until it is found

                    //current = iter.next(); // Jump > after name
                    let start_pos: usize = current.unwrap().0;

                    let mut pattern = String::with_capacity(name.len() + 3);
                    pattern.push_str("</");
                    pattern.push_str(&name);
                    pattern.push_str(">");

                    let search_result = find_pattern(iter, &pattern);

                    match search_result {
                        Ok(v) => {
                            return IteratorResultElement::Element {
                                element: Element::Named(NamedElement {
                                    name,
                                    attributes,
                                    child: Child::Text(
                                        &source[v.search_start_pos..{
                                            if v.result.is_some() {
                                                v.result.unwrap().0
                                            } else {
                                                source.len()
                                            }
                                        } - pattern.len()
                                            + 1],
                                    ),
                                }),
                                last_char: '>',
                            };
                        }
                        Err(c) => {
                            panic!(
                                "Reached EOF searching end tag for tag {} at pos {}",
                                name,
                                start_pos - name.len() - 1
                            );
                        }
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

    if current.is_none() {
        panic!("Reached EOF while looking for attributes.")
    }

    // Add last attribute
    if !before_value_start && !in_string {
        attributes.push(Attribute::from_string(
            &source[current_attr_start.unwrap()..current.unwrap().0],
        ));
    }

    attributes
}

/// Searches for the given pattern using the iterator
/// # Arguments
/// * `iter` The iterator to use for searching. The following .next is the start point of the search
/// * `pattern` The pattern to search for
/// # Returns
/// On Success: A Result:Ok containing a PatternSearchResult
/// On failure: A string indicating what went wrong
fn find_pattern<'a>(
    iter: &mut impl Iterator<Item = (usize, char)>,
    pattern: &str,
) -> Result<PatternSearchResult, &'a str> {
    let mut pattern_iter = pattern.chars();
    let mut pattern_current = pattern_iter.next();
    let mut current = iter.next();
    let search_start_iter = current;

    while current.is_some() && pattern_current.is_some() {
        if current.unwrap().1 == pattern_current.unwrap() {
            pattern_current = pattern_iter.next();
        } else {
            // Reset
            pattern_iter = pattern.chars();
            pattern_current = pattern_iter.next();
        }
        if pattern_current.is_some() {
            // Only go to next if we aren't at the end
            current = iter.next();
        }
    }

    if pattern_current.is_none() {
        // Pattern was exhausted: success
        return Result::Ok(PatternSearchResult {
            search_start_pos: search_start_iter.unwrap().0,
            result: current,
        });
    } else {
        return Result::Err("Reached EOF searching for end tag pattern.");
    }
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
        while current.is_some() && WHITESPACE_SYMBOLS.contains(&current.unwrap().1) && !at_opening {
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
    while current.is_some()
        && (VALID_TAG_CHARS.contains(&current.unwrap().1) || current.unwrap().1 == '/')
    {
        current = iter.next();
    }
    name_end = match current {
        None => {
            return Result::Err("Name not completed before string end");
        }
        Some(v) => v.0,
    };

    return Result::Ok(TagExtractionResult {
        tag: &source[if at_opening {
            name_start - 1
        } else {
            name_start
        }..name_end],
        last_char_higher_than: current.is_some() && current.unwrap().1 == '>',
    });
}
