use crate::types::Face;
use crate::types::Rgb;

#[derive(Clone)]
pub struct SyntaxTheme {
    pub comment: Face,
    pub constant: Face,
    pub function: Face,
    pub keyword: Face,
    pub number: Face,
    pub operator: Face,
    pub punctuation: Face,
    pub string: Face,
    pub r#type: Face,
    pub variable: Face,
    pub variable_member: Face,
    pub module: Face,
    pub label: Face,
    pub character: Face,
    pub boolean: Face,
}

impl SyntaxTheme {
    pub fn dark() -> Self {
        Self {
            comment: Face { fg: Some(Rgb::new(92, 99, 112)), ..Face::default() },
            constant: Face { fg: Some(Rgb::new(209, 154, 102)), ..Face::default() },
            function: Face { fg: Some(Rgb::new(97, 175, 239)), ..Face::default() },
            keyword: Face { fg: Some(Rgb::new(198, 120, 221)), ..Face::default() },
            number: Face { fg: Some(Rgb::new(209, 154, 102)), ..Face::default() },
            operator: Face { fg: Some(Rgb::new(86, 182, 194)), ..Face::default() },
            punctuation: Face { fg: Some(Rgb::new(172, 178, 190)), ..Face::default() },
            string: Face { fg: Some(Rgb::new(152, 195, 121)), ..Face::default() },
            r#type: Face { fg: Some(Rgb::new(229, 192, 123)), ..Face::default() },
            variable: Face { fg: Some(Rgb::new(224, 108, 117)), ..Face::default() },
            variable_member: Face { fg: Some(Rgb::new(224, 108, 117)), ..Face::default() },
            module: Face { fg: Some(Rgb::new(229, 192, 123)), ..Face::default() },
            label: Face { fg: Some(Rgb::new(229, 192, 123)), ..Face::default() },
            character: Face { fg: Some(Rgb::new(152, 195, 121)), ..Face::default() },
            boolean: Face { fg: Some(Rgb::new(209, 154, 102)), ..Face::default() },
        }
    }

    pub fn light() -> Self {
        Self {
            comment: Face { fg: Some(Rgb::new(160, 161, 167)), ..Face::default() },
            constant: Face { fg: Some(Rgb::new(152, 104, 1)), ..Face::default() },
            function: Face { fg: Some(Rgb::new(64, 120, 242)), ..Face::default() },
            keyword: Face { fg: Some(Rgb::new(166, 38, 164)), ..Face::default() },
            number: Face { fg: Some(Rgb::new(152, 104, 1)), ..Face::default() },
            operator: Face { fg: Some(Rgb::new(1, 132, 188)), ..Face::default() },
            punctuation: Face { fg: Some(Rgb::new(56, 58, 66)), ..Face::default() },
            string: Face { fg: Some(Rgb::new(80, 161, 79)), ..Face::default() },
            r#type: Face { fg: Some(Rgb::new(193, 132, 1)), ..Face::default() },
            variable: Face { fg: Some(Rgb::new(228, 86, 73)), ..Face::default() },
            variable_member: Face { fg: Some(Rgb::new(228, 86, 73)), ..Face::default() },
            module: Face { fg: Some(Rgb::new(193, 132, 1)), ..Face::default() },
            label: Face { fg: Some(Rgb::new(193, 132, 1)), ..Face::default() },
            character: Face { fg: Some(Rgb::new(80, 161, 79)), ..Face::default() },
            boolean: Face { fg: Some(Rgb::new(152, 104, 1)), ..Face::default() },
        }
    }

    pub const fn face(&self, highlight: SyntaxHighlight) -> Face {
        match highlight {
            SyntaxHighlight::Comment => self.comment,
            SyntaxHighlight::Constant => self.constant,
            SyntaxHighlight::Function => self.function,
            SyntaxHighlight::Keyword => self.keyword,
            SyntaxHighlight::Number => self.number,
            SyntaxHighlight::Operator => self.operator,
            SyntaxHighlight::Punctuation => self.punctuation,
            SyntaxHighlight::String => self.string,
            SyntaxHighlight::Type => self.r#type,
            SyntaxHighlight::Variable => self.variable,
            SyntaxHighlight::VariableMember => self.variable_member,
            SyntaxHighlight::Module => self.module,
            SyntaxHighlight::Label => self.label,
            SyntaxHighlight::Character => self.character,
            SyntaxHighlight::Boolean => self.boolean,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxHighlight {
    Comment,
    Constant,
    Function,
    Keyword,
    Number,
    Operator,
    Punctuation,
    String,
    Type,
    Variable,
    VariableMember,
    Module,
    Label,
    Character,
    Boolean,
}

impl SyntaxHighlight {
    pub const ALL: [Self; 15] = [
        Self::Comment,
        Self::Constant,
        Self::Function,
        Self::Keyword,
        Self::Number,
        Self::Operator,
        Self::Punctuation,
        Self::String,
        Self::Type,
        Self::Variable,
        Self::VariableMember,
        Self::Module,
        Self::Label,
        Self::Character,
        Self::Boolean,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Comment => "comment",
            Self::Constant => "constant",
            Self::Function => "function",
            Self::Keyword => "keyword",
            Self::Number => "number",
            Self::Operator => "operator",
            Self::Punctuation => "punctuation",
            Self::String => "string",
            Self::Type => "type",
            Self::Variable => "variable",
            Self::VariableMember => "variable.member",
            Self::Module => "module",
            Self::Label => "label",
            Self::Character => "character",
            Self::Boolean => "boolean",
        }
    }
}
