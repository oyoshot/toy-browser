#[derive(Debug, PartialEq)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, PartialEq)]
pub enum Selector {
    Simple(SimpleSelector),
}

#[derive(Debug, PartialEq)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Length(f32, Unit),
    ColorValue(Color),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Px,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Copy for Color {}

pub type Specificity = (usize, usize, usize);

impl Selector {
    pub fn specificity(&self) -> Specificity {
        let Selector::Simple(ref simple) = *self;
        let a = simple.id.iter().count();
        let b = simple.class.len();
        let c = simple.tag_name.iter().count();
        (a, b, c)
    }
}

impl Value {
    pub fn to_px(&self) -> f32 {
        match *self {
            Value::Length(f, Unit::Px) => f,
            _ => 0.0,
        }
    }
}

pub fn parse(source: String) -> Stylesheet {
    let mut parser = Parser {
        pos: 0,
        input: source,
    };
    Stylesheet {
        rules: parser.parse_rules(),
    }
}

struct Parser {
    pos: usize,
    input: String,
}

impl Parser {
    fn parse_rules(&mut self) -> Vec<Rule> {
        let mut rules = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }
            rules.push(self.parse_rule());
        }
        rules
    }

    fn parse_rule(&mut self) -> Rule {
        Rule {
            selectors: self.parse_selectors(),
            declarations: self.parse_declarations(),
        }
    }

    fn parse_selectors(&mut self) -> Vec<Selector> {
        let mut selectors = Vec::new();
        loop {
            selectors.push(Selector::Simple(self.parse_simple_selector()));
            self.consume_whitespace();
            match self.next_char() {
                ',' => {
                    self.consume_char();
                    self.consume_whitespace();
                }
                '{' => break,
                c => panic!("Unexpected character {} in selector list", c),
            }
        }
        selectors.sort_by_key(|b| std::cmp::Reverse(b.specificity()));
        selectors
    }

    fn parse_simple_selector(&mut self) -> SimpleSelector {
        let mut selector = SimpleSelector {
            tag_name: None,
            id: None,
            class: Vec::new(),
        };
        while !self.eof() {
            match self.next_char() {
                '#' => {
                    self.consume_char();
                    selector.id = Some(self.parse_identifier());
                }
                '.' => {
                    self.consume_char();
                    selector.class.push(self.parse_identifier());
                }
                '*' => {
                    self.consume_char();
                }
                c if valid_identifier_char(c) => {
                    selector.tag_name = Some(self.parse_identifier());
                }
                _ => break,
            }
        }
        selector
    }

    fn parse_declarations(&mut self) -> Vec<Declaration> {
        assert_eq!(self.consume_char(), '{');
        let mut declaration = Vec::new();
        loop {
            self.consume_whitespace();
            if self.next_char() == '}' {
                self.consume_char();
                break;
            }
            declaration.push(self.parse_declaration());
        }
        declaration
    }

    fn parse_declaration(&mut self) -> Declaration {
        let property_type = self.parse_identifier();
        self.consume_whitespace();
        assert_eq!(self.consume_char(), ':');
        self.consume_whitespace();

        let value = self.parse_value();
        self.consume_whitespace();
        assert_eq!(self.consume_char(), ';');

        Declaration {
            name: property_type,
            value,
        }
    }

    fn parse_value(&mut self) -> Value {
        match self.next_char() {
            '0'..='9' => self.parse_length(),
            '#' => self.parse_color(),
            _ => Value::Keyword(self.parse_identifier()),
        }
    }

    fn parse_length(&mut self) -> Value {
        Value::Length(self.parse_float(), self.parse_unit())
    }

    fn parse_float(&mut self) -> f32 {
        let s = self.consume_while(|c| matches!(c, '0'..='9' | '.'));
        s.parse().unwrap()
    }

    fn parse_unit(&mut self) -> Unit {
        match &*self.parse_identifier().to_ascii_lowercase() {
            "px" => Unit::Px,
            _ => panic!("unrecognized unit"),
        }
    }

    fn parse_color(&mut self) -> Value {
        assert_eq!(self.consume_char(), '#');
        Value::ColorValue(Color {
            r: self.parse_hex_pair(),
            g: self.parse_hex_pair(),
            b: self.parse_hex_pair(),
            a: 255,
        })
    }

    fn parse_hex_pair(&mut self) -> u8 {
        let s = &self.input[self.pos..self.pos + 2];
        self.pos += 2;
        u8::from_str_radix(s, 16).unwrap()
    }

    fn parse_identifier(&mut self) -> String {
        self.consume_while(valid_identifier_char)
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    fn consume_while<F>(&mut self, test: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push(self.consume_char());
        }
        result
    }

    fn consume_char(&mut self) -> char {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, cur_char) = iter.next().unwrap();
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        cur_char
    }

    fn next_char(&mut self) -> char {
        self.input[self.pos..].chars().next().unwrap()
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

fn valid_identifier_char(c: char) -> bool {
    matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_id() {
        let source = String::from(
            r#"
        #foo {
            display: inline;
        }
        "#,
        );

        let expected = Stylesheet {
            rules: vec![Rule {
                selectors: vec![Selector::Simple(SimpleSelector {
                    class: vec![],
                    id: Some(String::from("foo")),
                    tag_name: None,
                })],
                declarations: vec![Declaration {
                    name: String::from("display"),
                    value: Value::Keyword(String::from("inline")),
                }],
            }],
        };
        assert_eq!(expected, parse(source));
    }

    #[test]
    fn test_parse_class() {
        let source = String::from(
            r#"
        .foo {
            display: inline;
        }
        "#,
        );

        let expected = Stylesheet {
            rules: vec![Rule {
                selectors: vec![Selector::Simple(SimpleSelector {
                    class: vec![String::from("foo")],
                    id: None,
                    tag_name: None,
                })],
                declarations: vec![Declaration {
                    name: String::from("display"),
                    value: Value::Keyword(String::from("inline")),
                }],
            }],
        };
        assert_eq!(expected, parse(source));
    }

    #[test]
    fn test_parse_multiple_selectors() {
        let source = String::from(
            r#"
        foo, bar {
            display: inline;
        }
        "#,
        );

        let expected = Stylesheet {
            rules: vec![Rule {
                selectors: vec![
                    Selector::Simple(SimpleSelector {
                        class: vec![],
                        id: None,
                        tag_name: Some(String::from("foo")),
                    }),
                    Selector::Simple(SimpleSelector {
                        class: vec![],
                        id: None,
                        tag_name: Some(String::from("bar")),
                    }),
                ],
                declarations: vec![Declaration {
                    name: String::from("display"),
                    value: Value::Keyword(String::from("inline")),
                }],
            }],
        };
        assert_eq!(expected, parse(source));
    }

    #[test]
    fn test_parse_multiple_declarations() {
        let source = String::from(
            r#"
        html {
            width: 600px;
            padding: 10px;
            border-width: 1px;
            margin: auto;
            background: #aabbcc;
        }
        "#,
        );

        let expected = Stylesheet {
            rules: vec![Rule {
                selectors: vec![Selector::Simple(SimpleSelector {
                    class: vec![],
                    id: None,
                    tag_name: Some(String::from("html")),
                })],
                declarations: vec![
                    Declaration {
                        name: String::from("width"),
                        value: Value::Length(600.0, Unit::Px),
                    },
                    Declaration {
                        name: String::from("padding"),
                        value: Value::Length(10.0, Unit::Px),
                    },
                    Declaration {
                        name: String::from("border-width"),
                        value: Value::Length(1.0, Unit::Px),
                    },
                    Declaration {
                        name: String::from("margin"),
                        value: Value::Keyword(String::from("auto")),
                    },
                    Declaration {
                        name: String::from("background"),
                        value: Value::ColorValue(Color {
                            r: 170,
                            g: 187,
                            b: 204,
                            a: 255,
                        }),
                    },
                ],
            }],
        };
        assert_eq!(expected, parse(source));
    }

    #[test]
    fn test_parse_multiple_rules() {
        let source = String::from(
            r#"
        h1, h2, h3 {
            margin: auto;
            background: #cc0000;
        }
        div.note {
            margin-bottom: 20px;
            padding: 10px;
        }
        "#,
        );

        let expected = Stylesheet {
            rules: vec![
                Rule {
                    selectors: vec![
                        Selector::Simple(SimpleSelector {
                            class: vec![],
                            id: None,
                            tag_name: Some(String::from("h1")),
                        }),
                        Selector::Simple(SimpleSelector {
                            class: vec![],
                            id: None,
                            tag_name: Some(String::from("h2")),
                        }),
                        Selector::Simple(SimpleSelector {
                            class: vec![],
                            id: None,
                            tag_name: Some(String::from("h3")),
                        }),
                    ],
                    declarations: vec![
                        Declaration {
                            name: String::from("margin"),
                            value: Value::Keyword(String::from("auto")),
                        },
                        Declaration {
                            name: String::from("background"),
                            value: Value::ColorValue(Color {
                                r: 204,
                                g: 0,
                                b: 0,
                                a: 255,
                            }),
                        },
                    ],
                },
                Rule {
                    selectors: vec![Selector::Simple(SimpleSelector {
                        class: vec![String::from("note")],
                        id: None,
                        tag_name: Some(String::from("div")),
                    })],
                    declarations: vec![
                        Declaration {
                            name: String::from("margin-bottom"),
                            value: Value::Length(20.0, Unit::Px),
                        },
                        Declaration {
                            name: String::from("padding"),
                            value: Value::Length(10.0, Unit::Px),
                        },
                    ],
                },
            ],
        };
        assert_eq!(expected, parse(source));
    }
}
