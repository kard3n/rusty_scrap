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

pub enum MutableElement<'a> {
    Named(MutableNamedElement<'a>),
    Text(MutableTextElement<'a>),
    Comment(MutableCommentElement<'a>),
    Root(Vec<MutableElement<'a>>),
}

pub struct MutableNamedElement<'a> {
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
    modification: ModificationType,
    new_text: Option<String>, // Contains a value if the element was modified
}

pub struct MutableCommentElement<'a> {
    pub text: &'a str,
    start: usize,
    end: usize,
    modification: ModificationType,
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
