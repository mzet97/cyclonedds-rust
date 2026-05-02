//! Simplified IDL parser for extracting type definitions from OMG IDL files.
//!
//! This parser handles the most common IDL constructs needed for DDS type generation:
//! - `module` scopes
//! - `struct` definitions
//! - `enum` definitions
//! - `union` definitions (with switch/case)
//! - `bitmask` definitions (with @bit_bound annotation)
//! - `typedef` aliases
//! - Basic field types and annotations (@key, @position, @id, @hash_id, etc.)

use std::collections::HashMap;

/// Represents a parsed IDL type definition.
#[derive(Debug, Clone)]
pub enum IdlType {
    Struct(IdlStruct),
    Enum(IdlEnum),
    Union(IdlUnion),
    Bitmask(IdlBitmask),
    Typedef(IdlTypedef),
}

/// A struct definition.
#[derive(Debug, Clone)]
pub struct IdlStruct {
    pub name: String,
    pub fields: Vec<IdlField>,
    pub annotations: Vec<IdlAnnotation>,
}

/// A field within a struct.
#[derive(Debug, Clone)]
pub struct IdlField {
    pub name: String,
    pub ty: IdlTypeRef,
    pub annotations: Vec<IdlAnnotation>,
}

/// An enum definition.
#[derive(Debug, Clone)]
pub struct IdlEnum {
    pub name: String,
    pub variants: Vec<IdlEnumVariant>,
}

/// An enum variant with an optional explicit discriminant value.
#[derive(Debug, Clone)]
pub struct IdlEnumVariant {
    pub name: String,
    pub value: Option<i64>,
}

/// A union definition.
#[derive(Debug, Clone)]
pub struct IdlUnion {
    pub name: String,
    pub discriminant_type: IdlTypeRef,
    pub cases: Vec<IdlUnionCase>,
    pub default_case: Option<IdlUnionCase>,
}

/// A single case within a union.
#[derive(Debug, Clone)]
pub struct IdlUnionCase {
    pub label_values: Vec<i64>,
    pub name: String,
    pub ty: IdlTypeRef,
}

/// A bitmask definition.
#[derive(Debug, Clone)]
pub struct IdlBitmask {
    pub name: String,
    pub bit_bound: u32,
    pub flags: Vec<String>,
}

/// A typedef definition.
#[derive(Debug, Clone)]
pub struct IdlTypedef {
    pub name: String,
    pub ty: IdlTypeRef,
}

/// Reference to a type - either a primitive/builtin or a named reference.
#[derive(Debug, Clone)]
pub enum IdlTypeRef {
    /// A primitive type (boolean, octet, long, etc.)
    Primitive(PrimitiveType),
    /// A string type, optionally bounded.
    String { bound: Option<u32> },
    /// A sequence type with an optional maximum bound.
    Sequence {
        element_type: Box<IdlTypeRef>,
        bound: Option<u32>,
    },
    /// A fixed-size array.
    Array {
        element_type: Box<IdlTypeRef>,
        size: u32,
    },
    /// A reference to a named type (struct, enum, typedef, etc.)
    Named(String),
}

/// Primitive IDL types and their mappings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Boolean,
    Octet,
    Char,
    Wchar,
    Short,
    UnsignedShort,
    Long,
    UnsignedLong,
    LongLong,
    UnsignedLongLong,
    Float,
    Double,
    LongDouble,
}

/// IDL annotation.
#[derive(Debug, Clone)]
pub struct IdlAnnotation {
    pub name: String,
    pub params: Vec<String>,
}

/// The result of parsing an IDL file: a list of module-scoped type collections.
#[derive(Debug, Clone)]
pub struct IdlFile {
    /// Top-level types (outside any module).
    pub types: Vec<IdlType>,
    /// Modules mapping module name to their contained types.
    pub modules: HashMap<String, Vec<IdlType>>,
}

//----------------------------------------------------------------------
// Parser implementation
//----------------------------------------------------------------------

/// Parse an IDL file content into structured type definitions.
pub fn parse_idl(input: &str) -> Result<IdlFile, String> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(&tokens);
    parser.parse_file()
}

//----------------------------------------------------------------------
// Tokenizer
//----------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    IntLit(i64),
    StrLit(String),
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `;`
    Semi,
    /// `<`
    AngleOpen,
    /// `>`
    AngleClose,
    /// `,`
    Comma,
    /// `@`
    At,
    /// `::`
    Scope,
    /// `...`
    Ellipsis,
    /// `=`
    Assign,
    /// `|`
    Pipe,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '/' => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    // Single-line comment
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        if c == '\n' {
                            chars.next();
                            break;
                        }
                        chars.next();
                    }
                } else if chars.peek() == Some(&'*') {
                    // Multi-line comment
                    chars.next();
                    loop {
                        match chars.next() {
                            Some('*') if chars.peek() == Some(&'/') => {
                                chars.next();
                                break;
                            }
                            Some(_) => {}
                            None => return Err("Unterminated multi-line comment".into()),
                        }
                    }
                } else {
                    return Err("Unexpected character: /".into());
                }
            }
            '{' => {
                chars.next();
                tokens.push(Token::LBrace);
            }
            '}' => {
                chars.next();
                tokens.push(Token::RBrace);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '[' => {
                chars.next();
                tokens.push(Token::LBracket);
            }
            ']' => {
                chars.next();
                tokens.push(Token::RBracket);
            }
            ';' => {
                chars.next();
                tokens.push(Token::Semi);
            }
            '<' => {
                chars.next();
                tokens.push(Token::AngleOpen);
            }
            '>' => {
                chars.next();
                tokens.push(Token::AngleClose);
            }
            ',' => {
                chars.next();
                tokens.push(Token::Comma);
            }
            '@' => {
                chars.next();
                tokens.push(Token::At);
            }
            '|' => {
                chars.next();
                tokens.push(Token::Pipe);
            }
            '=' => {
                chars.next();
                tokens.push(Token::Assign);
            }
            ':' => {
                chars.next();
                if chars.peek() == Some(&':') {
                    chars.next();
                    tokens.push(Token::Scope);
                }
                // standalone ':' is ignored (used in IDL union switch syntax)
            }
            '.' => {
                chars.next();
                if chars.peek() == Some(&'.') {
                    chars.next();
                    if chars.peek() == Some(&'.') {
                        chars.next();
                        tokens.push(Token::Ellipsis);
                    }
                }
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => {
                            if let Some(c) = chars.next() {
                                s.push(c);
                            }
                        }
                        Some(c) => s.push(c),
                        None => return Err("Unterminated string literal".into()),
                    }
                }
                tokens.push(Token::StrLit(s));
            }
            c if c.is_ascii_digit()
                || (c == '-' && chars.peek().map_or(false, |nc| nc.is_ascii_digit())) =>
            {
                let negative = c == '-';
                if negative {
                    chars.next();
                }
                let mut num = String::new();
                if negative {
                    num.push('-');
                }
                // Handle hex prefix
                // Consume '0' prefix if present
                if chars.peek() == Some(&'0') {
                    num.push('0');
                    chars.next();
                }
                let is_hex = chars.peek() == Some(&'x') || chars.peek() == Some(&'X');
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit()
                        || (is_hex
                            && (c == 'x'
                                || c == 'X'
                                || ('a'..='f').contains(&c)
                                || ('A'..='F').contains(&c)))
                    {
                        num.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                // Handle L/LL/U/UL/ULL suffixes
                while let Some(&c) = chars.peek() {
                    if c == 'L' || c == 'l' || c == 'U' || c == 'u' {
                        chars.next();
                    } else {
                        break;
                    }
                }
                let val = if num.starts_with("-") {
                    num[1..]
                        .parse::<i64>()
                        .map(|v| -v)
                        .map_err(|e| format!("Invalid integer: {}", e))?
                } else {
                    num.parse::<i64>()
                        .map_err(|e| format!("Invalid integer: {}", e))?
                };
                tokens.push(Token::IntLit(val));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut ident = String::new();
                ident.push(ch);
                chars.next();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Ident(ident));
            }
            _ => {
                return Err(format!("Unexpected character: '{}'", ch));
            }
        }
    }

    Ok(tokens)
}

//----------------------------------------------------------------------
// Recursive descent parser
//----------------------------------------------------------------------

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        match self.advance() {
            Some(t) if t == expected => Ok(()),
            Some(t) => Err(format!("Expected {:?}, got {:?}", expected, t)),
            None => Err(format!("Expected {:?}, got end of input", expected)),
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s.clone()),
            Some(t) => Err(format!("Expected identifier, got {:?}", t)),
            None => Err("Expected identifier, got end of input".into()),
        }
    }

    fn expect_int(&mut self) -> Result<i64, String> {
        match self.advance() {
            Some(Token::IntLit(v)) => Ok(*v),
            Some(t) => Err(format!("Expected integer, got {:?}", t)),
            None => Err("Expected integer, got end of input".into()),
        }
    }

    fn parse_file(&mut self) -> Result<IdlFile, String> {
        let mut file = IdlFile {
            types: Vec::new(),
            modules: HashMap::new(),
        };

        while self.peek().is_some() {
            // Collect any annotations that precede a definition
            let annotations = self.parse_annotations()?;

            match self.peek() {
                Some(Token::Ident(kw)) => match kw.as_str() {
                    "module" => {
                        let (name, module_types) = self.parse_module()?;
                        file.modules.insert(name, module_types);
                    }
                    "struct" => {
                        file.types.push(self.parse_struct(annotations)?);
                    }
                    "enum" => {
                        file.types.push(self.parse_enum(annotations)?);
                    }
                    "union" => {
                        file.types.push(self.parse_union(annotations)?);
                    }
                    "bitmask" => {
                        file.types.push(self.parse_bitmask(annotations)?);
                    }
                    "typedef" => {
                        if let Some(td) = self.parse_typedef(annotations)? {
                            file.types.push(td);
                        }
                    }
                    "const" => {
                        // Skip const declarations
                        self.skip_to_semi()?;
                    }
                    "import" | "include" | "type" | "annotation" => {
                        // Skip these top-level declarations
                        self.skip_to_semi()?;
                    }
                    _ => {
                        // Try to skip unknown constructs
                        self.skip_to_semi()?;
                    }
                },
                Some(Token::At) => {
                    // Annotations are parsed at the top of the loop
                    // This shouldn't happen normally, but handle gracefully
                }
                _ => {
                    self.advance();
                }
            }
        }

        Ok(file)
    }

    fn parse_annotations(&mut self) -> Result<Vec<IdlAnnotation>, String> {
        let mut annotations = Vec::new();
        loop {
            if self.peek() == Some(&Token::At) {
                self.advance(); // consume @
                let name = self.expect_ident()?;
                let params = if self.peek() == Some(&Token::LParen) {
                    self.advance(); // consume (
                    let mut ps = Vec::new();
                    while self.peek() != Some(&Token::RParen) && self.peek().is_some() {
                        match self.advance() {
                            Some(Token::Ident(s)) => ps.push(s.clone()),
                            Some(Token::IntLit(v)) => ps.push(v.to_string()),
                            Some(Token::StrLit(s)) => ps.push(s.clone()),
                            _ => {}
                        }
                    }
                    self.expect(&Token::RParen)?;
                    ps
                } else {
                    Vec::new()
                };
                annotations.push(IdlAnnotation { name, params });
            } else {
                break;
            }
        }
        Ok(annotations)
    }

    fn parse_module(&mut self) -> Result<(String, Vec<IdlType>), String> {
        self.expect(&Token::Ident("module".into()))?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut types = Vec::new();
        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
            let annotations = self.parse_annotations()?;
            match self.peek() {
                Some(Token::Ident(kw)) => match kw.as_str() {
                    "struct" => {
                        types.push(self.parse_struct(annotations)?);
                    }
                    "enum" => {
                        types.push(self.parse_enum(annotations)?);
                    }
                    "union" => {
                        types.push(self.parse_union(annotations)?);
                    }
                    "bitmask" => {
                        types.push(self.parse_bitmask(annotations)?);
                    }
                    "typedef" => {
                        if let Some(td) = self.parse_typedef(annotations)? {
                            types.push(td);
                        }
                    }
                    "const" | "import" | "include" | "type" | "annotation" => {
                        self.skip_to_semi()?;
                    }
                    _ => {
                        self.skip_to_semi()?;
                    }
                },
                _ => {
                    self.advance();
                }
            }
        }

        self.expect(&Token::RBrace)?;
        Ok((name, types))
    }

    fn parse_struct(&mut self, annotations: Vec<IdlAnnotation>) -> Result<IdlType, String> {
        self.expect(&Token::Ident("struct".into()))?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut fields = Vec::new();
        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
            let field_annotations = self.parse_annotations()?;
            let (field_name, field_type) = self.parse_field()?;
            fields.push(IdlField {
                name: field_name,
                ty: field_type,
                annotations: field_annotations,
            });
        }

        self.expect(&Token::RBrace)?;
        // Optional trailing semicolon
        if self.peek() == Some(&Token::Semi) {
            self.advance();
        }

        Ok(IdlType::Struct(IdlStruct {
            name,
            fields,
            annotations,
        }))
    }

    fn parse_field(&mut self) -> Result<(String, IdlTypeRef), String> {
        let ty = self.parse_type_ref()?;
        let name = self.expect_ident()?;

        // Handle array suffix: name[N]
        let final_type = if self.peek() == Some(&Token::LBracket) {
            self.advance();
            let size = self.expect_int()? as u32;
            self.expect(&Token::RBracket)?;
            IdlTypeRef::Array {
                element_type: Box::new(ty),
                size,
            }
        } else {
            ty
        };

        self.expect(&Token::Semi)?;
        Ok((name, final_type))
    }

    fn parse_type_ref(&mut self) -> Result<IdlTypeRef, String> {
        match self.peek() {
            Some(Token::Ident(kw)) => match kw.as_str() {
                "boolean" => {
                    self.advance();
                    Ok(IdlTypeRef::Primitive(PrimitiveType::Boolean))
                }
                "octet" => {
                    self.advance();
                    Ok(IdlTypeRef::Primitive(PrimitiveType::Octet))
                }
                "char" => {
                    self.advance();
                    Ok(IdlTypeRef::Primitive(PrimitiveType::Char))
                }
                "wchar" => {
                    self.advance();
                    Ok(IdlTypeRef::Primitive(PrimitiveType::Wchar))
                }
                "short" => {
                    self.advance();
                    Ok(IdlTypeRef::Primitive(PrimitiveType::Short))
                }
                "unsigned" => {
                    self.advance();
                    match self.peek() {
                        Some(Token::Ident(s)) if s == "short" => {
                            self.advance();
                            Ok(IdlTypeRef::Primitive(PrimitiveType::UnsignedShort))
                        }
                        Some(Token::Ident(s)) if s == "long" => {
                            self.advance();
                            // Check for "long long"
                            if self.peek() == Some(&Token::Ident("long".into())) {
                                self.advance();
                                Ok(IdlTypeRef::Primitive(PrimitiveType::UnsignedLongLong))
                            } else {
                                Ok(IdlTypeRef::Primitive(PrimitiveType::UnsignedLong))
                            }
                        }
                        _ => Err("Expected 'short' or 'long' after 'unsigned'".into()),
                    }
                }
                "long" => {
                    self.advance();
                    // Check for "long long"
                    if self.peek() == Some(&Token::Ident("long".into())) {
                        self.advance();
                        Ok(IdlTypeRef::Primitive(PrimitiveType::LongLong))
                    } else {
                        Ok(IdlTypeRef::Primitive(PrimitiveType::Long))
                    }
                }
                "float" => {
                    self.advance();
                    Ok(IdlTypeRef::Primitive(PrimitiveType::Float))
                }
                "double" => {
                    self.advance();
                    Ok(IdlTypeRef::Primitive(PrimitiveType::Double))
                }
                "string" => {
                    self.advance();
                    let bound = if self.peek() == Some(&Token::AngleOpen) {
                        self.advance();
                        let b = self.expect_int()? as u32;
                        self.expect(&Token::AngleClose)?;
                        Some(b)
                    } else {
                        None
                    };
                    Ok(IdlTypeRef::String { bound })
                }
                "wstring" => {
                    self.advance();
                    // Skip optional bound
                    if self.peek() == Some(&Token::AngleOpen) {
                        self.advance();
                        let _b = self.expect_int()?;
                        self.expect(&Token::AngleClose)?;
                    }
                    Ok(IdlTypeRef::String { bound: None })
                }
                "sequence" => {
                    self.advance();
                    self.expect(&Token::AngleOpen)?;
                    let elem_type = self.parse_type_ref()?;
                    let bound = if self.peek() == Some(&Token::Comma) {
                        self.advance();
                        Some(self.expect_int()? as u32)
                    } else {
                        None
                    };
                    self.expect(&Token::AngleClose)?;
                    Ok(IdlTypeRef::Sequence {
                        element_type: Box::new(elem_type),
                        bound,
                    })
                }
                // Named type reference
                _ => {
                    let name = self.expect_ident()?;
                    // Handle scoped names like Foo::Bar
                    let mut full_name = name;
                    while self.peek() == Some(&Token::Scope) {
                        self.advance();
                        let next = self.expect_ident()?;
                        full_name = format!("{}::{}", full_name, next);
                    }
                    Ok(IdlTypeRef::Named(full_name))
                }
            },
            Some(Token::Scope) => {
                // Scoped name starting with ::
                self.advance();
                let name = self.expect_ident()?;
                let mut full_name = format!("::{}", name);
                while self.peek() == Some(&Token::Scope) {
                    self.advance();
                    let next = self.expect_ident()?;
                    full_name = format!("{}::{}", full_name, next);
                }
                Ok(IdlTypeRef::Named(full_name))
            }
            _ => Err("Expected type name".into()),
        }
    }

    fn parse_enum(&mut self, _annotations: Vec<IdlAnnotation>) -> Result<IdlType, String> {
        self.expect(&Token::Ident("enum".into()))?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut variants = Vec::new();
        let mut next_value: i64 = 0;

        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
            let vname = self.expect_ident()?;
            let value = if self.peek() == Some(&Token::Assign) {
                self.advance();
                let v = self.expect_int()?;
                next_value = v + 1;
                Some(v)
            } else {
                let v = next_value;
                next_value += 1;
                Some(v)
            };
            variants.push(IdlEnumVariant { name: vname, value });

            // Optional comma
            if self.peek() == Some(&Token::Comma) {
                self.advance();
            }
        }

        self.expect(&Token::RBrace)?;
        // Optional trailing semicolon
        if self.peek() == Some(&Token::Semi) {
            self.advance();
        }

        Ok(IdlType::Enum(IdlEnum { name, variants }))
    }

    fn parse_union(&mut self, _annotations: Vec<IdlAnnotation>) -> Result<IdlType, String> {
        self.expect(&Token::Ident("union".into()))?;
        let name = self.expect_ident()?;
        self.expect(&Token::Ident("switch".into()))?;
        self.expect(&Token::LParen)?;
        let disc_type = self.parse_type_ref()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;

        let mut cases = Vec::new();
        let mut default_case = None;

        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
            if self.peek() == Some(&Token::Ident("case".into())) {
                self.advance();
                let mut label_values = Vec::new();
                // Parse case label(s) until ':'
                loop {
                    // Handle negative values
                    let val = self.expect_int()?;
                    label_values.push(val);
                    if self.peek() == Some(&Token::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                // Skip the ':'
                // The tokenizer doesn't produce a colon token (it only produces Scope for '::')
                // The colon is actually already consumed or was never tokenized.
                // Let's just try to consume it if present as an ident or skip.

                let (field_name, field_type) = self.parse_field()?;
                cases.push(IdlUnionCase {
                    label_values,
                    name: field_name,
                    ty: field_type,
                });
            } else if self.peek() == Some(&Token::Ident("default".into())) {
                self.advance();
                // Skip the ':'
                let (field_name, field_type) = self.parse_field()?;
                default_case = Some(IdlUnionCase {
                    label_values: Vec::new(),
                    name: field_name,
                    ty: field_type,
                });
            } else {
                self.advance();
            }
        }

        self.expect(&Token::RBrace)?;
        // Optional trailing semicolon
        if self.peek() == Some(&Token::Semi) {
            self.advance();
        }

        Ok(IdlType::Union(IdlUnion {
            name,
            discriminant_type: disc_type,
            cases,
            default_case,
        }))
    }

    fn parse_bitmask(&mut self, annotations: Vec<IdlAnnotation>) -> Result<IdlType, String> {
        self.expect(&Token::Ident("bitmask".into()))?;
        let name = self.expect_ident()?;

        // Extract bit_bound from annotations
        let bit_bound = annotations
            .iter()
            .find(|a| a.name == "bit_bound")
            .and_then(|a| a.params.first())
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(32);

        self.expect(&Token::LBrace)?;

        let mut flags = Vec::new();
        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
            // Skip any position annotation
            let _ = self.parse_annotations()?;
            let fname = self.expect_ident()?;
            flags.push(fname);
            if self.peek() == Some(&Token::Comma) {
                self.advance();
            }
        }

        self.expect(&Token::RBrace)?;
        // Optional trailing semicolon
        if self.peek() == Some(&Token::Semi) {
            self.advance();
        }

        Ok(IdlType::Bitmask(IdlBitmask {
            name,
            bit_bound,
            flags,
        }))
    }

    fn parse_typedef(
        &mut self,
        _annotations: Vec<IdlAnnotation>,
    ) -> Result<Option<IdlType>, String> {
        self.expect(&Token::Ident("typedef".into()))?;
        let ty = self.parse_type_ref()?;
        let name = self.expect_ident()?;

        // Handle array typedef: typedef long MyArray[10];
        let final_type = if self.peek() == Some(&Token::LBracket) {
            self.advance();
            let size = self.expect_int()? as u32;
            self.expect(&Token::RBracket)?;
            IdlTypeRef::Array {
                element_type: Box::new(ty),
                size,
            }
        } else {
            ty
        };

        self.expect(&Token::Semi)?;
        Ok(Some(IdlType::Typedef(IdlTypedef {
            name,
            ty: final_type,
        })))
    }

    fn skip_to_semi(&mut self) -> Result<(), String> {
        let mut depth = 0;
        while let Some(tok) = self.advance() {
            match tok {
                Token::LBrace | Token::LParen => depth += 1,
                Token::RBrace | Token::RParen => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                Token::Semi if depth == 0 => return Ok(()),
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_struct() {
        let idl = r#"
            struct Point {
                long x;
                long y;
                double z;
            };
        "#;
        let file = parse_idl(idl).unwrap();
        assert_eq!(file.types.len(), 1);
        match &file.types[0] {
            IdlType::Struct(s) => {
                assert_eq!(s.name, "Point");
                assert_eq!(s.fields.len(), 3);
                assert_eq!(s.fields[0].name, "x");
            }
            _ => panic!("Expected struct"),
        }
    }

    #[test]
    fn test_parse_enum() {
        let idl = r#"
            enum Color {
                Red,
                Green,
                Blue = 10
            };
        "#;
        let file = parse_idl(idl).unwrap();
        match &file.types[0] {
            IdlType::Enum(e) => {
                assert_eq!(e.name, "Color");
                assert_eq!(e.variants.len(), 3);
                assert_eq!(e.variants[2].value, Some(10));
            }
            _ => panic!("Expected enum"),
        }
    }

    #[test]
    fn test_parse_module() {
        let idl = r#"
            module MyModule {
                struct Inner {
                    string name;
                };
            };
        "#;
        let file = parse_idl(idl).unwrap();
        assert!(file.modules.contains_key("MyModule"));
        let mod_types = file.modules.get("MyModule").unwrap();
        assert_eq!(mod_types.len(), 1);
    }

    #[test]
    fn test_parse_bitmask() {
        let idl = r#"
            @bit_bound(8)
            bitmask StatusFlags {
                Ready,
                Active,
                Error
            };
        "#;
        let file = parse_idl(idl).unwrap();
        match &file.types[0] {
            IdlType::Bitmask(b) => {
                assert_eq!(b.name, "StatusFlags");
                assert_eq!(b.bit_bound, 8);
                assert_eq!(b.flags.len(), 3);
            }
            _ => panic!("Expected bitmask"),
        }
    }

    #[test]
    fn test_parse_sequence_field() {
        let idl = r#"
            struct Data {
                sequence<long> values;
                sequence<string, 10> bounded_strs;
            };
        "#;
        let file = parse_idl(idl).unwrap();
        match &file.types[0] {
            IdlType::Struct(s) => {
                assert_eq!(s.fields.len(), 2);
                match &s.fields[0].ty {
                    IdlTypeRef::Sequence { bound, .. } => assert_eq!(*bound, None),
                    _ => panic!("Expected sequence"),
                }
                match &s.fields[1].ty {
                    IdlTypeRef::Sequence { bound, .. } => assert_eq!(*bound, Some(10)),
                    _ => panic!("Expected bounded sequence"),
                }
            }
            _ => panic!("Expected struct"),
        }
    }

    #[test]
    fn test_parse_key_annotation() {
        let idl = r#"
            struct KeyedData {
                @key long id;
                string name;
            };
        "#;
        let file = parse_idl(idl).unwrap();
        match &file.types[0] {
            IdlType::Struct(s) => {
                assert!(s.fields[0].annotations.iter().any(|a| a.name == "key"));
                assert!(!s.fields[1].annotations.iter().any(|a| a.name == "key"));
            }
            _ => panic!("Expected struct"),
        }
    }

    #[test]
    fn test_parse_nested_struct() {
        let idl = r#"
            struct Point {
                double x;
                double y;
            };
            struct Pose {
                Point position;
                Point orientation;
            };
        "#;
        let file = parse_idl(idl).unwrap();
        assert_eq!(file.types.len(), 2);
        match &file.types[1] {
            IdlType::Struct(s) => {
                assert_eq!(s.name, "Pose");
                assert_eq!(s.fields.len(), 2);
                match &s.fields[0].ty {
                    IdlTypeRef::Named(name) => assert_eq!(name, "Point"),
                    _ => panic!("Expected named type reference"),
                }
            }
            _ => panic!("Expected struct"),
        }
    }

    #[test]
    fn test_parse_cross_module_reference() {
        let idl = r#"
            module Geometry {
                struct Point {
                    double x;
                    double y;
                };
            };
            struct Pose {
                Geometry::Point position;
            };
        "#;
        let file = parse_idl(idl).unwrap();
        assert_eq!(file.types.len(), 1);
        assert!(file.modules.contains_key("Geometry"));
        match &file.types[0] {
            IdlType::Struct(s) => match &s.fields[0].ty {
                IdlTypeRef::Named(name) => assert_eq!(name, "Geometry::Point"),
                _ => panic!("Expected scoped named type"),
            },
            _ => panic!("Expected struct"),
        }
    }

    #[test]
    fn test_parse_typedef_array() {
        let idl = r#"
            typedef long IntArray[10];
            struct Data {
                IntArray values;
            };
        "#;
        let file = parse_idl(idl).unwrap();
        assert_eq!(file.types.len(), 2);
        match &file.types[0] {
            IdlType::Typedef(td) => {
                assert_eq!(td.name, "IntArray");
                match &td.ty {
                    IdlTypeRef::Array { element_type, size } => {
                        assert_eq!(*size, 10);
                        match element_type.as_ref() {
                            IdlTypeRef::Primitive(PrimitiveType::Long) => {}
                            _ => panic!("Expected long array"),
                        }
                    }
                    _ => panic!("Expected array typedef"),
                }
            }
            _ => panic!("Expected typedef"),
        }
    }
}
