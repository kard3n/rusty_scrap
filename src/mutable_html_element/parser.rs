use crate::mutable_html_element::{Attribute, Child, ModificationType, MutableCommentElement, MutableElement, MutableNamedElement, MutableTextElement, TerminalValue};
use crate::constants::{*};
struct TagExtractionResult<'a> {
    tag: &'a str,
    search_start: usize,
    search_end: usize,
    last_char_higher_than: bool,
}

struct AttributeExtractionResult<'a> {
    attributes: Vec<Attribute<'a>>,
    search_start: usize, // position where the search started
    search_end: usize,   // position where the search ended
}

struct PatternSearchResult {
    result: Option<(usize, char)>, // Position and char of the last iteration
    search_start: usize,
    search_end: usize,
}

enum IteratorResultElement<'a> {
    Element {
        element: MutableElement<'a>,
        last_char: char,
        last_pos: usize,
    },
    ClosingTag {
        name: &'a str,
        last_pos: usize,
    },
    End,
}


impl<'a> MutableElement<'a> {
    pub fn from_string(source: &'a str) -> Self {
        let mut iter = source.char_indices();
        let mut result: Vec<MutableElement<'a>> = Vec::new();

        let mut current_result: IteratorResultElement;
        let mut last_char_lower_than = false;
        let mut stop: bool = false;

        while !stop {
            current_result = Self::from_iterator(&mut iter, &source, last_char_lower_than);
            match current_result {
                IteratorResultElement::Element {
                    element,
                    last_char,
                    last_pos,
                } => {
                    result.push(element);
                    last_char_lower_than = last_char == '<';
                }
                IteratorResultElement::ClosingTag { name, last_pos } => {
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
                // Text element reaches EOF
                None => IteratorResultElement::Element {
                    element: MutableElement::Text(MutableTextElement {
                        text: &source[start_pos..],
                        start: start_pos,
                        end: source.len(),
                        modification: ModificationType::NONE,
                        new_text: String::new(),
                    }),
                    last_char: '_',
                    last_pos: source.len(),
                },
                // Text element stops before EOF
                Some(v) => IteratorResultElement::Element {
                    element: MutableElement::Text(MutableTextElement {
                        text: &source[start_pos..v.0],
                        start: start_pos,
                        end: v.0,
                        modification: ModificationType::NONE,
                        new_text: String::new(),
                    }),
                    last_char: v.1,
                    last_pos: v.0,
                },
            }
        } else {
            let element_start: usize = current.unwrap().0;
            let tag_extraction_result = extract_tag(&source, iter, true);
            let name: &str = match &tag_extraction_result {
                Ok(v) => v.tag,
                Err(c) => {
                    panic!("{}", c);
                }
            };

            /*
            // Named element
            // First is a <, go to next
            if !last_char_lower_than {
                current = iter.next();
            }
             */

            if name.starts_with("/") {
                // Closing tag of normal element

                return IteratorResultElement::ClosingTag {
                    name: &source[current.unwrap().0..{
                        while current.unwrap().1 != '>' {
                            // Go to closing symbol
                            current = iter.next()
                        }
                        current.unwrap().0
                    }],
                    last_pos: current.unwrap().0,
                };
            } else {
                // Opening tag of an element

                // Special case: comment
                // Don't need to read attributes, but look for special closing -->
                if name.starts_with("!--") {
                    // Comment
                    let search_result = find_pattern(iter, "-->");

                    match search_result {
                        Result::Ok(result) => {
                            return IteratorResultElement::Element {
                                element: MutableElement::Comment(MutableCommentElement {
                                    text: &source[element_start + name.len()
                                        ..if result.result.is_some() {
                                        result.result.unwrap().0 - 2
                                    } else {
                                        source.len()
                                    }],
                                    start: element_start,
                                    end: if result.result.is_some() {
                                        result.result.unwrap().0
                                    } else {
                                        source.len()
                                    },
                                    modification: ModificationType::ADD,
                                    new_text: "".to_string(),
                                }),
                                last_char: '>',
                                last_pos: if result.result.is_some() {
                                    result.result.unwrap().0
                                } else {
                                    source.len()
                                },
                            };
                        }
                        Result::Err(c) => {
                            panic!("{}", c);
                        }
                    }
                }

                let attributes = match &tag_extraction_result {
                    Ok(v) => {
                        if v.last_char_higher_than {
                            AttributeExtractionResult {
                                attributes: Vec::new(),
                                search_start: v.search_start,
                                //tag extraction ended with closing symbol, so attribute search would have ended there too
                                search_end: v.search_end,
                            }
                        } else {
                            extract_attributes(&source, iter)
                        }
                    }
                    Err(c) => {
                        panic!("{}", c);
                    }
                };
                let mut child_nodes: Vec<MutableElement<'a>> = Vec::new();

                if name.starts_with('!') {
                    // Doctype
                    return IteratorResultElement::Element {
                        element: MutableElement::Named(MutableNamedElement{
                            name,
                            attributes: attributes.attributes,
                            child: Child::None,
                    }),
                        last_char: '_',
                        last_pos: attributes.search_end,
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
                        Ok(search_result_ok) => {
                            return IteratorResultElement::Element {
                                element: MutableElement::Named(MutableNamedElement {
                                    name,
                                    attributes: attributes.attributes,
                                    child: Child::Text(
                                        &source[search_result_ok.search_start..{
                                            if search_result_ok.result.is_some() {
                                                search_result_ok.result.unwrap().0
                                            } else {
                                                source.len()
                                            }
                                        } - pattern.len()
                                            + 1],
                                    ),
                                }),
                                last_char: '>',
                                last_pos: search_result_ok.search_end,
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
                } else if SELF_CLOSING_TAGS.contains(&name) {
                    // Self closing tag
                    return IteratorResultElement::Element {
                        element: MutableElement::Named(MutableNamedElement {
                            name,
                            attributes: attributes.attributes,
                            child: if !child_nodes.is_empty() {
                                Child::Nodes(child_nodes)
                            } else {
                                Child::None
                            },
                        }),
                        last_char: '_',
                        last_pos: attributes.search_end,
                    };
                } else {
                    // Not a self-cosing tag, therefore can have children
                    let mut next_child = Self::from_iterator(iter, source, false);
                    let mut last_char_lower_than;
                    while !(matches!(next_child, IteratorResultElement::ClosingTag { .. })
                        || matches!(next_child, IteratorResultElement::End))
                    {
                        last_char_lower_than = false;
                        match next_child {
                            IteratorResultElement::Element {
                                element,
                                last_char,
                                last_pos,
                            } => {
                                child_nodes.push(element);
                                last_char_lower_than = last_char == '<';
                            }
                            _ => {}
                        }

                        next_child = Self::from_iterator(iter, &source, last_char_lower_than);
                    }

                    // Check that the element was closed properly
                    match next_child {
                        IteratorResultElement::ClosingTag { name, last_pos } => {
                            if name != &name[1..] {
                                panic!(
                                    "Name of the closing tag ('{}') does not match name of the opening tag ('{}') at {}.",
                                    &name,
                                    name,
                                    element_start - 1
                                )
                            };

                            return IteratorResultElement::Element {
                                element: MutableElement::Named(MutableNamedElement {
                                    name,
                                    attributes: attributes.attributes,
                                    child: if !child_nodes.is_empty() {
                                        Child::Nodes(child_nodes)
                                    } else {
                                        Child::None
                                    },
                                }),
                                last_char: '_',
                                last_pos,
                            };
                        }
                        _ => {
                            panic!(
                                "Element '{}' at position {} did not have a closing tag!",
                                name,
                                element_start - 1
                            );
                        }
                    };
                }
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
) -> AttributeExtractionResult<'a> {
    // Find area containing the attributes

    let mut previous_was_escape: bool = false;
    let mut in_string: bool = false;

    let mut attributes: Vec<Attribute<'a>> = Vec::new();

    let mut current_attr_start: Option<usize> = None;
    let mut after_equal_sign: bool = false;
    let mut before_value_start: bool = true;

    let mut current = iter.next();

    if current.is_none() {
        panic!("Attribute search started at EOF.")
    }

    let first_pos = current.unwrap().0;

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

    AttributeExtractionResult {
        attributes,
        search_start: first_pos,
        search_end: current.unwrap().0,
    }
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
    let mut previous_current = current;
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
            previous_current = iter.next();
            current = iter.next();
        }
    }

    if pattern_current.is_none() {
        // Pattern was exhausted: success
        return Result::Ok(PatternSearchResult {
            result: current,
            search_start: search_start_iter.unwrap().0,
            search_end: if current.is_some() {
                current.unwrap().0
            } else if previous_current.is_some() {
                previous_current.unwrap().0
            } else {
                return Err("Started search for pattern at EOF.");
            },
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
    let search_start = current.unwrap().0;
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
        search_start,
        search_end: current.unwrap().0,
        last_char_higher_than: current.is_some() && current.unwrap().1 == '>',
    });
}