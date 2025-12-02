use crate::mutable_html_element::{Child, ModificationType, MutableElement, ParsedHTML};
use std::collections::LinkedList;

// Recursive, using the DOM tree
// For the current element:
// 1: Add opening tag (incl. attributes) to the string (modified or not), starting at ending position of previous call/recursion
// 2: Apply function to all child elements. Every call returns the usize of the last char written by it. The next one gets passed said char, starts writing after it.
// 3: Add closing tag.
// 4: Return position of last written character

impl ParsedHTML<'_> {
    /// Converts the passed element to its responding String representation
    /// # Parameters
    /// * start_pos: the position at which the first character should be written
    /// * element: the element to convert to a string
    /// * result: reference to the result string which should be written to
    pub fn to_html_string(
        &self,
        start_pos: usize,
        element: &MutableElement,
        result: &mut String,
    ) -> usize {
        match &element {
            MutableElement::Named(_) => {}
            MutableElement::SelfClosing(_) => {}
            MutableElement::Text(t) => {
                // Is just a string
                match t.modification_type {
                    ModificationType::ADD => {
                        result.push_str(&t.new_text.as_ref().unwrap().to_string());
                        return start_pos + t.new_text.as_ref().unwrap().len();
                    }
                    ModificationType::DELETE => return start_pos,
                    ModificationType::EDIT => {
                        result.push_str(&t.new_text.as_ref().unwrap().to_string());
                        return start_pos + t.new_text.as_ref().unwrap().len() - t.text.len();
                    }
                    ModificationType::NONE => {
                        result.push_str(t.text);
                    }
                }
            }
            MutableElement::Comment(c) => {
                match c.modification_type {
                    ModificationType::ADD | ModificationType::EDIT => {
                        result.push_str("<--");
                        result.push_str(&c.new_text.as_ref().unwrap().to_string());
                        result.push_str("-->");
                        return start_pos + c.new_text.as_ref().unwrap().len() + 6;
                    }
                    ModificationType::DELETE => {
                        return start_pos;
                    }
                    ModificationType::NONE => {
                        result.push_str(&self.original_content[c.start..c.end]);
                        return start_pos + c.end - c.start;
                    }
                }
            }
            MutableElement::Root(_) => {}
        }

        return 0;
    }

    /// Calculates the size of the resulting string using the changes
    pub fn calculate_new_size(&self, changes: &LinkedList<&MutableElement>) -> usize {
        let mut new_size = self.original_content.len();

        for change in changes {
            match &change {
                MutableElement::Named(n) => {
                    match &n.modification_type {
                        ModificationType::ADD => {
                            // name + open/closing symbols
                            new_size += n.new_name.as_ref().unwrap().len() + 5;
                        }
                        ModificationType::DELETE => {
                            new_size -= n.closing_end - n.opening_start;
                        }
                        ModificationType::EDIT => {
                            new_size += n.new_name.as_ref().unwrap().len() - n.name.len();
                        }
                        _other => {}
                    }
                }
                MutableElement::SelfClosing(sc) => match &sc.modification_type {
                    ModificationType::ADD => {
                        new_size += sc.new_name.as_ref().unwrap().len();
                    }
                    ModificationType::DELETE => {
                        new_size -= sc.end - sc.start;
                    }
                    ModificationType::EDIT => {
                        new_size += sc.new_name.as_ref().unwrap().len() - (sc.end - sc.start);
                    }
                    _other => {}
                },
                MutableElement::Text(t) => match &t.modification_type {
                    ModificationType::ADD => new_size += t.new_text.as_ref().unwrap().len(),
                    ModificationType::DELETE => new_size -= t.end - t.start,
                    ModificationType::EDIT => {
                        new_size += t.new_text.as_ref().unwrap().len() - (t.end - t.start)
                    }
                    _other => {}
                },
                MutableElement::Comment(c) => match &c.modification_type {
                    ModificationType::ADD => {
                        new_size += c.new_text.as_ref().unwrap().len();
                    }
                    ModificationType::DELETE => {
                        new_size -= c.end - c.start;
                    }
                    ModificationType::EDIT => {
                        new_size += c.new_text.as_ref().unwrap().len() - (c.end - c.start);
                    }
                    _other => {}
                },
                MutableElement::Root(r) => {
                    // TODO
                }
            }
        }

        return new_size;
    }
}

impl MutableElement<'_> {
    /// Returns a list of all elements with changes
    fn get_changes(&'_ self) -> LinkedList<&'_ MutableElement<'_>> {
        let mut changes: LinkedList<&MutableElement> = LinkedList::new();

        match &self {
            MutableElement::Named(n) => {
                // Adds itself if it was changed
                match &n.modification_type {
                    ModificationType::NONE => {}
                    _other => changes.push_back(self),
                }

                // Adds changes from all child elements
                match &n.child {
                    Child::None => {}
                    Child::Nodes(nodes) => {
                        for elem in nodes {
                            changes.append(&mut elem.get_changes())
                        }
                    }
                    Child::Text(t) => match &t.modification_type {
                        ModificationType::NONE => {}
                        _other => changes.push_back(self),
                    },
                }
            }
            MutableElement::Text(t) => match &t.modification_type {
                ModificationType::NONE => {}
                _other => changes.push_back(self),
            },
            MutableElement::Comment(c) => match &c.modification_type {
                ModificationType::NONE => {}
                _other => changes.push_back(self),
            },
            MutableElement::Root(r) => {
                for elem in r {
                    changes.append(&mut elem.get_changes())
                }
            }
            &MutableElement::SelfClosing(sc) => match &sc.modification_type {
                ModificationType::NONE => {}
                _other => changes.push_back(self),
            },
        }

        return changes;
    }
}
