use std::collections::*;
use std::fmt::*;

use crate::block::*;
use crate::data::*;

#[derive(Clone)]
pub struct RuleMap {
    pub rule_map: HashMap<String, Box<Rule>>,
    pub start_rule_id: String,
}

impl RuleMap {
    pub fn new(block_map: Vec<BlockMap>, start_rule_id: String) -> BlockParseResult<RuleMap> {
        let rule_map = RuleMap {
            rule_map: RuleMap::to_rule_map(block_map)?,
            start_rule_id: start_rule_id,
        };

        return Ok(rule_map);
    }

    fn to_rule_map(block_maps: Vec<BlockMap>) -> BlockParseResult<HashMap<String, Box<Rule>>> {
        let mut rule_map = HashMap::<String, Box<Rule>>::new();

        for each_block_map in block_maps {
            for (_, each_block) in each_block_map {
                for each_cmd in each_block.cmds {
                    match each_cmd {
                        BlockCommand::Define { pos: _, rule } => {
                            rule_map.insert(rule.id.clone(), Box::new(rule));
                        },
                        _ => (),
                    }
                }
            }
        }

        return Ok(rule_map);
    }
}

impl Display for RuleMap {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut rule_text_lines = Vec::<String>::new();

        for each_rule in self.rule_map.values() {
            rule_text_lines.push(each_rule.to_string());
        }

        return writeln!(f, "{}", rule_text_lines.join("\n"));
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub enum RuleElementLookaheadKind {
    None,
    Positive,
    Negative,
}

impl RuleElementLookaheadKind {
    // ret: 文字がマッチしなければ RuleElementLookaheadKind::None
    pub fn new(value: &str) -> RuleElementLookaheadKind {
        return match value {
            "&" => RuleElementLookaheadKind::Positive,
            "!" => RuleElementLookaheadKind::Negative,
            _ => RuleElementLookaheadKind::None,
        }
    }

    pub fn is_none(&self) -> bool {
        return *self == RuleElementLookaheadKind::None;
    }
}

impl Display for RuleElementLookaheadKind {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let s = match self {
            RuleElementLookaheadKind::None => "",
            RuleElementLookaheadKind::Positive => "&",
            RuleElementLookaheadKind::Negative => "!",
        };

        return write!(f, "{}", s);
    }
}

#[derive(Clone)]
pub struct Rule {
    pub pos: CharacterPosition,
    pub id: String,
    pub name: String,
    pub generics_arg_ids: Vec<String>,
    pub func_arg_ids: Vec<String>,
    pub group: Box<RuleGroup>,
}

impl Rule {
    pub fn new(pos: CharacterPosition, id: String, name: String, generics_arg_ids: Vec<String>, func_arg_ids: Vec<String>, group: Box<RuleGroup>) -> Rule {
        return Rule {
            pos: pos,
            id: id,
            name: name,
            generics_arg_ids: generics_arg_ids,
            func_arg_ids: func_arg_ids,
            group: group,
        };
    }
}

impl Display for Rule {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let generics_arg_id_text = if self.generics_arg_ids.len() == 0 {
            String::new()
        } else {
            format!("({})", self.generics_arg_ids.iter().map(|s| format!("${}", s)).collect::<Vec<String>>().join(", "))
        };

        return write!(f, "{}{} <- {}", self.name, generics_arg_id_text, self.group);
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub enum Infinitable<T: Clone + Display + PartialEq + PartialOrd> {
    Normal(T),
    Infinite,
}

impl<T: Clone + Display + PartialEq + PartialOrd> Infinitable<T> {
    pub fn is_infinite(&self) -> bool {
        return *self == Infinitable::<T>::Infinite;
    }
}

impl<T: Clone + Display + PartialEq + PartialOrd> Display for Infinitable<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Infinitable::Normal(v) => v.to_string(),
            Infinitable::Infinite => "Infinite".to_string(),
        };

        return write!(f, "{}", s);
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub struct RuleElementLoopCount {
    pub min: usize,
    pub max: Infinitable<usize>,
}

impl RuleElementLoopCount {
    pub fn new(min: usize, max: Infinitable<usize>) -> RuleElementLoopCount {
        return RuleElementLoopCount {
            min: min,
            max: max,
        };
    }

    pub fn from_symbol(value: &str) -> RuleElementLoopCount {
        return match value {
            "?" => RuleElementLoopCount::new(0, Infinitable::Normal(1)),
            "*" => RuleElementLoopCount::new(0, Infinitable::Infinite),
            "+" => RuleElementLoopCount::new(1, Infinitable::Infinite),
            _ => RuleElementLoopCount::new(1, Infinitable::Normal(1)),
        }
    }

    pub fn get_single_loop() -> RuleElementLoopCount {
        return RuleElementLoopCount::new(1, Infinitable::Normal(1));
    }

    pub fn is_single_loop(&self) -> bool {
        return self.min == 1 && self.max == Infinitable::Normal(1);
    }

    pub fn to_string(&self, is_loop_count: bool, prefix: &str, separator: &str, suffix: &str) -> String {
        if self.is_single_loop() {
            return String::new();
        }

        if is_loop_count {
            match self.to_tuple() {
                (0, 1) => return "?".to_string(),
                (0, -1) => return "*".to_string(),
                (1, -1) => return "+".to_string(),
                _ => (),
            }
        }

        let min_count = if self.min == 0 { String::new() } else { self.min.to_string() };
        let max_count = match self.max { Infinitable::Normal(max_num) => max_num.to_string(), Infinitable::Infinite => String::new(), };
        return format!("{}{}{}{}{}", prefix, min_count, separator, max_count, suffix);
    }

    pub fn to_tuple(&self) -> (usize, i32) {
        let max_num = match self.max {
            Infinitable::Normal(num) => num as i32,
            Infinitable::Infinite => -1,
        };

        return (self.min, max_num)
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub enum RuleElementOrder {
    Random(RuleElementLoopCount),
    Sequential,
}

impl RuleElementOrder {
    pub fn is_random(&self) -> bool {
        return *self != RuleElementOrder::Sequential;
    }
}

impl Display for RuleElementOrder {
    fn fmt(&self, f: &mut Formatter) -> Result {
        return match self {
            RuleElementOrder::Random(loop_count) => write!(f, "Random({})", loop_count.to_string(true, "{", ",", "}")),
            RuleElementOrder::Sequential => write!(f, "Sequential"),
        }
    }
}

#[derive(Clone)]
pub enum RuleElement {
    Group(Box<RuleGroup>),
    Expression(Box<RuleExpression>),
}

impl Display for RuleElement {
    fn fmt(&self, f: &mut Formatter) -> Result {
        return match self {
            RuleElement::Group(group) => write!(f, "{}", group),
            RuleElement::Expression(expr) => write!(f, "{}", expr),
        }
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub enum RuleGroupKind {
    Choice,
    Sequence,
}

impl Display for RuleGroupKind {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let s = match self {
            RuleGroupKind::Choice => "Choice",
            RuleGroupKind::Sequence => "Sequence",
        };

        return write!(f, "{}", s);
    }
}

#[derive(Clone)]
pub struct RuleGroup {
    pub kind: RuleGroupKind,
    pub sub_elems: Vec<RuleElement>,
    pub ast_reflection_style: ASTReflectionStyle,
    pub lookahead_kind: RuleElementLookaheadKind,
    pub loop_count: RuleElementLoopCount,
    pub elem_order: RuleElementOrder,
}

impl RuleGroup {
    pub fn new(kind: RuleGroupKind) -> RuleGroup {
        return RuleGroup {
            kind: kind,
            sub_elems: vec![],
            lookahead_kind: RuleElementLookaheadKind::None,
            loop_count: RuleElementLoopCount::get_single_loop(),
            ast_reflection_style: ASTReflectionStyle::Reflection(String::new()),
            elem_order: RuleElementOrder::Sequential,
        };
    }

    pub fn extract(&self) -> RuleGroup {
        return match self.sub_elems.get(0) {
            Some(child_elem) if self.sub_elems.len() == 1 => {
                match child_elem {
                    RuleElement::Group(group) => group.clone().extract(),
                    _ => self.clone(),
                }
            },
            _ => self.clone(),
        };
    }
}

impl Display for RuleGroup {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut seq_text = Vec::<String>::new();

        for each_elem in &self.sub_elems {
            match each_elem {
                RuleElement::Group(each_group) => {
                    seq_text.push(each_group.to_string());
                },
                RuleElement::Expression(each_expr) => {
                    seq_text.push(format!("{}", each_expr));
                },
            }
        }

        let separator = match self.kind {
            RuleGroupKind::Choice => {
                match self.elem_order {
                    RuleElementOrder::Random(_) => ", ",
                    RuleElementOrder::Sequential => " : ",
                }
            },
            RuleGroupKind::Sequence => " ",
        };
        let loop_text = self.loop_count.to_string(true, "{", ",", "}");
        let order_text = match &self.elem_order { RuleElementOrder::Random(loop_count) => loop_count.to_string(false, "^[", "-", "]"), RuleElementOrder::Sequential => String::new(), };

        return write!(f, "{}", format!("{}({}){}{}{}", self.lookahead_kind, seq_text.join(separator), loop_text, order_text, self.ast_reflection_style));
    }
}

#[derive(Clone)]
pub enum RuleExpressionKind {
    ArgID,
    CharClass,
    Func(Vec<Box<RuleGroup>>),
    Generics(Vec<Box<RuleGroup>>),
    ID,
    String,
    Wildcard,
}

impl Display for RuleExpressionKind {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let s = match self {
            RuleExpressionKind::ArgID => "ArgID",
            RuleExpressionKind::CharClass => "CharClass",
            RuleExpressionKind::Func(_) => "Func",
            RuleExpressionKind::Generics(_) => "Generics",
            RuleExpressionKind::ID => "ID",
            RuleExpressionKind::String => "String",
            RuleExpressionKind::Wildcard => "Wildcard",
        };

        write!(f, "{}", s)
    }
}

#[derive(Clone)]
pub struct RuleExpression {
    pub pos: CharacterPosition,
    pub kind: RuleExpressionKind,
    pub value: String,
    pub ast_reflection_style: ASTReflectionStyle,
    pub lookahead_kind: RuleElementLookaheadKind,
    pub loop_count: RuleElementLoopCount,
}

impl RuleExpression {
    pub fn new(pos: CharacterPosition, kind: RuleExpressionKind, value: String) -> RuleExpression {
        return RuleExpression {
            pos: pos,
            kind: kind,
            value: value,
            ast_reflection_style: ASTReflectionStyle::NoReflection,
            lookahead_kind: RuleElementLookaheadKind::None,
            loop_count: RuleElementLoopCount::get_single_loop(),
        }
    }
}

impl Display for RuleExpression {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let loop_text = self.loop_count.to_string(true, "{", ",", "}");
        let value_text = match self.kind.clone() {
            RuleExpressionKind::ArgID => format!("${}", self.value),
            RuleExpressionKind::CharClass => self.value.clone(),
            RuleExpressionKind::Func(args) => {
                let arg_text = args.iter().map(|each_arg| each_arg.to_string()).collect::<Vec<String>>();
                format!("{}({})", self.value, arg_text.join(", "))
            },
            RuleExpressionKind::Generics(args) => {
                let arg_text = args.iter().map(|each_arg| each_arg.to_string()).collect::<Vec<String>>();
                format!("{}<{}>", self.value, arg_text.join(", "))
            },
            RuleExpressionKind::ID => self.value.clone(),
            RuleExpressionKind::String => format!("\"{}\"", self.value),
            RuleExpressionKind::Wildcard => ".".to_string(),
        };

        return write!(f, "{}{}{}{}", self.lookahead_kind, value_text, loop_text, self.ast_reflection_style);
    }
}
