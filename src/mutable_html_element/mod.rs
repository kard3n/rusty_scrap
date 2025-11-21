pub mod parser;
pub mod serializer;
mod test;

#[derive(Debug)]
pub enum TerminalValue<'a> {
    Int(i32),
    Float(f32),
    Str(&'a str),
}

pub enum ModificationType {
    ADD,    // Element was added
    DELETE, // Element was deleted. Do not add to output
    EDIT,   // Element was edited -> its content changed
    NONE,   // Element was not modified
}

pub struct ParsedHTML<'a> {
    original_content: String,
    pub parsed_content: MutableElement<'a>,
}

impl ParsedHTML<'_> {
    /// Creates a new ParsedHTML from a given string
    pub fn from_string(input: &'_ str) -> ParsedHTML<'_> {
        return ParsedHTML {
            original_content: input.to_string(),
            parsed_content: MutableElement::from_string(&input),
        };
    }
}

pub enum MutableElement<'a> {
    Named(MutableNamedElement<'a>),
    SelfClosing(MutableSelfClosingNamedElement<'a>),
    Text(MutableTextElement<'a>),
    Comment(MutableCommentElement<'a>),
    Root(Vec<MutableElement<'a>>),
}

/// A mutable, non self-closing element
pub struct MutableNamedElement<'a> {
    pub name: &'a str,
    pub attributes: Vec<Attribute<'a>>, // TODO: make modifiable
    pub child: Child<'a>,
    modification_type: ModificationType,
    new_name: Option<String>,
    opening_start: usize,
    opening_end: usize,
    closing_start: usize,
    closing_end: usize,
}

/// A mutable self-closing element
pub struct MutableSelfClosingNamedElement<'a> {
    pub name: &'a str,
    pub attributes: Vec<Attribute<'a>>, // TODO: make modifiable
    pub child: Child<'a>,
    modification_type: ModificationType,
    new_name: Option<String>,
    start: usize,
    end: usize,
}

pub struct MutableTextElement<'a> {
    pub text: &'a str,
    start: usize,
    end: usize,
    modification_type: ModificationType,
    new_text: Option<String>, // Contains a value if the element was modified
}

pub struct MutableCommentElement<'a> {
    pub text: &'a str,
    start: usize,
    end: usize,
    modification_type: ModificationType,
    new_text: Option<String>, // Contains a value if the element was modified
}

pub struct MutableRootElement<'a> {
    pub elements: Vec<MutableElement<'a>>,
}

pub enum Child<'a> {
    None,                           // Does not have a child
    Nodes(Vec<MutableElement<'a>>), // One or more children
    Text(&'a str),                  // Text content, like in <script>
}

// TODO: add stand and end
pub struct Attribute<'a> {
    pub name: &'a str,
    pub value: TerminalValue<'a>,
}
