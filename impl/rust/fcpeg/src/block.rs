use std::collections::*;

use crate::*;
use crate::data::*;
use crate::parser::*;
use crate::rule::*;

use rustnutlib::console::*;

macro_rules! block_map {
    ($($block_name:expr => $func_name:ident), *,) => {
        {
            let mut block_map = BlockMap::new();
            $(block_map.insert($block_name.to_string(), FCPEGBlock::$func_name());)*
            block_map
        }
    };
}

macro_rules! block {
    ($block_name:expr, $cmds:expr) => {
        Block::new($block_name.to_string(), $cmds)
    };
}

macro_rules! use_block {
    ($block_name:expr) => {
        BlockCommand::Use(0, "".to_string(), $block_name.to_string(), $block_name.to_string())
    };
}

macro_rules! rule {
    ($rule_name:expr, $($choice:expr), *,) => {
        {
            let choices = vec![$(
                match $choice {
                    RuleElementContainer::RuleChoice(v) => v,
                    _ => panic!(),
                }
            )*];

            let rule = Rule::new($rule_name.to_string(), vec![], choices);
            BlockCommand::Define(0, rule)
        }
    };
}

macro_rules! start_cmd {
    ($file_alias_name:expr, $block_name:expr, $rule_name:expr) => {
        BlockCommand::Start(0, $file_alias_name.to_string(), $block_name.to_string(), $rule_name.to_string())
    };
}

macro_rules! choice {
    () => {
        RuleChoice {
            elem_containers: vec![],
            lookahead_kind: RuleLookaheadKind::None,
            loop_count: (1, 1),
            ast_reflection: ASTReflection::Unreflectable,
            is_random_order: false,
            occurrence_count: (1, 1),
            has_choices: false,
        }
    };

    ($options:expr, $($sub_elem:expr), *,) => {
        {
            let mut choice = choice!();
            choice.elem_containers = vec![$($sub_elem,)*];
            choice.ast_reflection = ASTReflection::Reflectable("".to_string());

            for opt in $options {
                match opt {
                    "&" | "!" => choice.lookahead_kind = RuleLookaheadKind::to_lookahead_kind(opt),
                    "?" | "*" | "+" => choice.loop_count = RuleCountConverter::loop_symbol_to_count(opt),
                    "#" => choice.ast_reflection = ASTReflection::Unreflectable,
                    "##" => choice.ast_reflection = ASTReflection::Expandable,
                    ":" => choice.has_choices = true,
                    _ => panic!(),
                }
            }

            // $(choice.$field_name = $field_value;)*
            RuleElementContainer::RuleChoice(Box::new(choice))
        }
    };
}

macro_rules! expr {
    ($kind:ident) => {
        RuleExpression {
            line: 0,
            kind: RuleExpressionKind::$kind,
            lookahead_kind: RuleLookaheadKind::None,
            loop_count: (1, 1),
            ast_reflection: ASTReflection::Unreflectable,
            value: "".to_string(),
        }
    };

    ($kind:ident, $value:expr $(, $option:expr) *) => {
        {
            let mut expr = expr!($kind);
            expr.value = $value.to_string();

            let leaf_name = match RuleExpressionKind::$kind {
                RuleExpressionKind::ID => $value.to_string(),
                _ => "".to_string(),
            };

            expr.ast_reflection = ASTReflection::Reflectable(leaf_name);

            $(
                match $option {
                    "&" | "!" => expr.lookahead_kind = RuleLookaheadKind::to_lookahead_kind($option),
                    "?" | "*" | "+" => expr.loop_count = RuleCountConverter::loop_symbol_to_count($option),
                    "#" => expr.ast_reflection = ASTReflection::Unreflectable,
                    _ => panic!(),
                }
            )*

            RuleElementContainer::RuleExpression(Box::new(expr))
        }
    };
}

pub type BlockMap = HashMap<String, Block>;

#[derive(PartialOrd, PartialEq, Debug, Clone)]
pub enum BlockTokenKind {
    ID,
    Number,
    Space,
    String,
    StringInBracket,
    Symbol,
}

impl std::fmt::Display for BlockTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BlockTokenKind::ID => return write!(f, "ID"),
            BlockTokenKind::Number => return write!(f, "Number"),
            BlockTokenKind::Space => return write!(f, "Space"),
            BlockTokenKind::String => return write!(f, "String"),
            BlockTokenKind::StringInBracket => return write!(f, "StringInBracket"),
            BlockTokenKind::Symbol => return write!(f, "Symbol"),
        }
    }
}

#[derive(Clone)]
pub struct BlockToken {
    pub line: usize,
    pub kind: BlockTokenKind,
    pub value: String,
}

impl BlockToken {
    pub fn new(line: usize, kind: BlockTokenKind, value: String) -> BlockToken {
        return BlockToken {
            line: line,
            kind: kind,
            value: value,
        }
    }
}

#[derive(Debug)]
pub enum BlockParseError {
    Unknown(),
    BlockAliasNotFound(usize, String),
    DuplicatedBlockAliasName(usize, String),
    DuplicatedBlockName(usize, String),
    DuplicatedStartCmd(),
    ExpectedBlockDef(usize),
    ExpectedToken(usize, String),
    InternalErr(String),
    InvalidCharClassFormat(usize, String),
    InvalidToken(usize, String),
    MainBlockNotFound(),
    NoChoiceOrExpressionContent(usize),
    NoStartCmdInMainBlock(),
    RuleHasNoChoice(String),
    RuleInMainBlock(),
    StartCmdOutsideMainBlock(),
    TooBigNumber(usize, String),
    UnexpectedEOF(usize, String),
    UnexpectedToken(usize, String, String),
    UnknownPragmaName(usize, String),
    UnknownSyntax(usize, String),
    UnknownToken(usize, String),
}

impl BlockParseError {
    pub fn get_log_data(&self) -> ConsoleLogData {
        match self {
            BlockParseError::Unknown() => ConsoleLogData::new(ConsoleLogKind::Error, "unknown error", vec![], vec![]),
            BlockParseError::BlockAliasNotFound(line, block_alias_name) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("block alias '{}' not found", block_alias_name), vec![format!("line:\t{}", line + 1)], vec![]),
            BlockParseError::DuplicatedBlockAliasName(line, block_alias_name) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("duplicated block alias name '{}'", block_alias_name), vec![format!("line:\t{}", line + 1)], vec![]),
            BlockParseError::DuplicatedBlockName(line, block_name) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("duplicated block name '{}'", block_name), vec![format!("line:\t{}", line + 1)], vec![]),
            BlockParseError::DuplicatedStartCmd() => ConsoleLogData::new(ConsoleLogKind::Error, "duplicated start command", vec![], vec![]),
            BlockParseError::ExpectedBlockDef(line) => ConsoleLogData::new(ConsoleLogKind::Error, "expected block definition", vec![format!("line:\t{}", line + 1)], vec![]),
            BlockParseError::ExpectedToken(line, expected_str) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("expected token {}", expected_str), vec![format!("line:\t{}", line + 1)], vec![]),
            BlockParseError::InternalErr(err_name) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("internal error: {}", err_name), vec![], vec![]),
            BlockParseError::InvalidCharClassFormat(line, value) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("invalid character class format '{}'", value), vec![format!("line:\t{}", line + 1)], vec![]),
            BlockParseError::InvalidToken(line, value) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("invalid token '{}'", value), vec![format!("line:\t{}", line + 1)], vec![]),
            BlockParseError::MainBlockNotFound() => ConsoleLogData::new(ConsoleLogKind::Error, "main block not found", vec![], vec![]),
            BlockParseError::NoChoiceOrExpressionContent(line) => ConsoleLogData::new(ConsoleLogKind::Error, "no choice or expression content", vec![format!("line:\t{}", line + 1)], vec![]),
            BlockParseError::NoStartCmdInMainBlock() => ConsoleLogData::new(ConsoleLogKind::Error, "no start command in main block", vec![], vec![]),
            BlockParseError::RuleHasNoChoice(rule_name) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("rule '{}' has no choice", rule_name), vec![], vec![]),
            BlockParseError::RuleInMainBlock() => ConsoleLogData::new(ConsoleLogKind::Error, "rule in main block", vec![], vec![]),
            BlockParseError::StartCmdOutsideMainBlock() => ConsoleLogData::new(ConsoleLogKind::Error, "start command outside main block", vec![], vec![]),
            BlockParseError::TooBigNumber(line, number) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("too big number {}", number), vec![format!("line:\t{}", line + 1)], vec![]),
            BlockParseError::UnexpectedEOF(line, expected_str) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("unexpected EOF, expected {}", expected_str), vec![format!("line:\t{}", line + 1)], vec![]),
            BlockParseError::UnexpectedToken(line, unexpected_token, expected_str) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("unexpected token '{}', expected {}", unexpected_token, expected_str), vec![format!("line:\t{}", line + 1)], vec![]),
            BlockParseError::UnknownPragmaName(line, unknown_pragma_name) => ConsoleLogData::new(ConsoleLogKind::Error, "unknown pragma name", vec![format!("line:\t{}", line + 1), format!("pragma name:\t{}", unknown_pragma_name)], vec![]),
            BlockParseError::UnknownSyntax(line, target_token) => ConsoleLogData::new(ConsoleLogKind::Error, "unknown syntax", vec![format!("line: {}", line + 1), format!("target token:\t'{}'", target_token)], vec![]),
            BlockParseError::UnknownToken(line, unknown_token) => ConsoleLogData::new(ConsoleLogKind::Error, &format!("unknown token '{}'", unknown_token), vec![format!("line:\t{}", line + 1)], vec![]),
        }
    }
}

pub struct BlockParser {}

impl BlockParser {
    // note: FileMan から最終的な RuleMap を取得する
    pub fn get_rule_map(fcpeg_file_man: &mut FCPEGFileMan) -> Result<RuleMap, SyntaxParseError> {
        let mut tmp_file_man = FCPEGFileMan::new("".to_string(), "".to_string());
        tmp_file_man.block_map = FCPEGBlock::get_block_map();
        let mut fcpeg_rule_map = RuleMap::new(".Syntax.FCPEG".to_string());
        match fcpeg_rule_map.add_rules_from_fcpeg_file_man(&tmp_file_man) { Ok(()) => (), Err(e) => { let mut cons = Console::new(); cons.log(e.get_log_data(), false); panic!(); } };
        let mut parser = SyntaxParser::new(fcpeg_rule_map)?;

        BlockParser::set_block_map_to_all_files(&mut parser, fcpeg_file_man)?;

        let mut rule_map = RuleMap::new("Main".to_string());
        rule_map.add_rules_from_fcpeg_file_man(fcpeg_file_man).unwrap();

        return Ok(rule_map);
    }

    // note: 全ファイルに BlockMap を設定する
    fn set_block_map_to_all_files(parser: &mut SyntaxParser, fcpeg_file_man: &mut FCPEGFileMan) -> Result<(), SyntaxParseError> {
        let tree = BlockParser::to_syntax_tree(parser, fcpeg_file_man)?;
        fcpeg_file_man.block_map = BlockParser::to_block_map(&tree)?;

        for sub_file_man in fcpeg_file_man.sub_file_aliase_map.values_mut() {
            BlockParser::set_block_map_to_all_files(parser, sub_file_man)?;
        }

        return Ok(());
    }

    // note: ブロックマップとファイルを元に 1 ファイルの FCPEG コードの構文木を取得する
    fn to_syntax_tree(parser: &mut SyntaxParser, fcpeg_file_man: &FCPEGFileMan) -> Result<SyntaxTree, SyntaxParseError> {
        let tree = parser.get_syntax_tree(fcpeg_file_man.fcpeg_file_content.clone())?;

        println!("print {}", fcpeg_file_man.fcpeg_file_content);
        tree.print(true);

        return Ok(tree);
    }

    // note: FCPEG コードの構文木 → ブロックマップの変換
    fn to_block_map(tree: &SyntaxTree) -> Result<BlockMap, SyntaxParseError> {
        let mut block_map = BlockMap::new();

        let root = tree.clone_child();

        let block_nodes = root.get_node_list()?.get_node_list_child(0)?.filter_unreflectable_out();

        for each_block_elem in &block_nodes {
            let each_block_node_list = each_block_elem.get_node_list()?;
            let block_name = each_block_node_list.get_node_list_child(0)?.to_string();

            let mut cmds = Vec::<BlockCommand>::new();
            let cmd_elems = each_block_node_list.get_node_list_child(1)?.filter_unreflectable_out();

            for each_cmd_elem in &cmd_elems {
                let each_cmd_node_list = each_cmd_elem.get_node_list()?.get_node_list_child(0)?;
                let new_cmd = BlockParser::to_block_cmd(each_cmd_node_list)?;
                cmds.push(new_cmd);
            }

            block_map.insert(block_name.clone(), Block::new(block_name.clone(), cmds));
        }

        block_map.insert("".to_string(), Block::new("Main".to_string(), vec![]));

        for (_, each_block) in &block_map {
            each_block.print();
            println!();
        }

        return Ok(block_map);
    }

    fn to_block_cmd(cmd_node_list: &SyntaxNodeList) -> Result<BlockCommand, SyntaxParseError> {
        return match &cmd_node_list.ast_reflection {
            ASTReflection::Reflectable(node_name) => match node_name.as_str() {
                "DefineCmd" => BlockParser::to_define_cmd(cmd_node_list),
                _ => Err(SyntaxParseError::InvalidSyntaxTreeStruct(format!("invalid node name '{}'", node_name))),
            },
            _ => Err(SyntaxParseError::InvalidSyntaxTreeStruct("invalid operation".to_string())),
        };
    }

    fn to_define_cmd(cmd_node_list: &SyntaxNodeList) -> Result<BlockCommand, SyntaxParseError> {
        let rule_name = cmd_node_list.get_node_list_child(0)?.to_string();
        let choice_node_list = cmd_node_list.get_node_list_child(1)?;
        let mut choices = Vec::<Box::<RuleChoice>>::new();

        let new_choice = BlockParser::to_rule_choice_elem(choice_node_list)?;
        choices.push(Box::new(new_choice));

        // todo: さらなる seq の解析

        let rule = Rule::new(rule_name, vec![], choices);
        return Ok(BlockCommand::Define(0, rule));
    }

    // note: Seq を解析する
    fn to_seq_elem(seq_node: &SyntaxNodeList) -> Result<RuleElementContainer, SyntaxParseError> {
        // todo: 先読みなどの処理
        let mut children = Vec::<RuleElementContainer>::new();

        // note: SeqElem ノードをループ
        for each_seq_elem_elem in &seq_node.filter_unreflectable_out() {
            let each_seq_elem_node = each_seq_elem_elem.get_node_list()?;

            // note: Lookahead ノード
            let lookahead_kind = match each_seq_elem_node.find_first_child_node(vec!["Lookahead"]) {
                Some(v) => {
                    match v.get_leaf_child(0)?.value.as_str() {
                        "&" => RuleLookaheadKind::Positive,
                        "!" => RuleLookaheadKind::Negative,
                        _ => return Err(SyntaxParseError::InvalidSyntaxTreeStruct(format!("unknown lookahead kind"))),
                    }
                },
                None => RuleLookaheadKind::None,
            };

            // note: Loop ノード
            let loop_count = match each_seq_elem_node.find_first_child_node(vec!["Loop"]) {
                Some(v) => {
                    match v.get_child(0)? {
                        SyntaxNodeElement::NodeList(node) => {
                            let min_num = match node.get_node_list_child(0)?.get_leaf_child(0)?.value.parse::<i32>() {
                                Ok(v) => v,
                                Err(_) => return Err(SyntaxParseError::InvalidSyntaxTreeStruct(format!("invalid minimum loop value"))),
                            };

                            let max_num = match node.get_node_list_child(1)?.get_leaf_child(0)?.value.parse::<i32>() {
                                Ok(v) => v,
                                Err(_) => return Err(SyntaxParseError::InvalidSyntaxTreeStruct(format!("invalid maximum loop value"))),
                            };

                            (min_num, max_num)
                        },
                        SyntaxNodeElement::Leaf(leaf) => {
                            match leaf.value.as_str() {
                                "?" => (0, 1),
                                "*" => (0, -1),
                                "+" => (1, -1),
                                _ => return Err(SyntaxParseError::InvalidSyntaxTreeStruct(format!("unknown lookahead kind"))),
                            }
                        }
                    }
                },
                None => (1, 1),
            };

            // note: ASTReflection ノード
            // todo: 構成ファイルによって切り替える
            let ast_reflection = match each_seq_elem_node.find_first_child_node(vec!["ASTReflection"]) {
                Some(v) => {
                    match v.get_node_list_child(0) {
                        Ok(v) => ASTReflection::from_config(true, v.to_string()),
                        Err(_) => ASTReflection::from_config(false, String::new()),
                    }
                },
                None => ASTReflection::from_config(false, String::new()),
            };

            // Choice または Expr ノード
            let choice_or_expr_node = match each_seq_elem_node.find_first_child_node(vec!["Choice", "Expr"]) {
                Some(v) => v,
                None => return Err(SyntaxParseError::InvalidSyntaxTreeStruct("invalid operation".to_string())),
            };

            match &choice_or_expr_node.ast_reflection {
                ASTReflection::Reflectable(name) => {
                    let new_elem = match name.as_str() {
                        "Choice" => {
                            let mut new_choice = BlockParser::to_rule_choice_elem(choice_or_expr_node.get_node_list_child(0)?)?;
                            new_choice.lookahead_kind = lookahead_kind;
                            new_choice.loop_count = loop_count;
                            new_choice.ast_reflection = ast_reflection;
                            RuleElementContainer::RuleChoice(Box::new(new_choice))
                        },
                        "Expr" => {
                            let mut new_expr = BlockParser::to_rule_expr_elem(choice_or_expr_node)?;
                            new_expr.lookahead_kind = lookahead_kind;
                            new_expr.loop_count = loop_count;
                            new_expr.ast_reflection = ast_reflection;
                            RuleElementContainer::RuleExpression(Box::new(new_expr))
                        }
                        _ => return Err(SyntaxParseError::InvalidSyntaxTreeStruct(format!("invalid node name '{}'", name))),
                    };

                    children.push(new_elem);
                },
                _ => return Err(SyntaxParseError::InvalidSyntaxTreeStruct("invalid operation".to_string())),
            };
        }

        let mut choice = RuleChoice::new(RuleLookaheadKind::None, (1, 1), ASTReflection::Unreflectable, false, (1, 1), false);
        choice.elem_containers = children;
        return Ok(RuleElementContainer::RuleChoice(Box::new(choice)));
    }

    // note: Rule.PureChoice ノードの解析
    fn to_rule_choice_elem(choice_node: &SyntaxNodeList) -> Result<RuleChoice, SyntaxParseError> {
        let mut children = Vec::<RuleElementContainer>::new();
        let mut has_choices = false;

        // Seq ノードをループ
        for seq_elem in &choice_node.filter_unreflectable_out() {
            match &seq_elem.as_ref() {
                SyntaxNodeElement::NodeList(node) => {
                    match &seq_elem.as_ref().get_ast_reflection() {
                        ASTReflection::Reflectable(name) => if name == "Seq" {
                            let new_child = BlockParser::to_seq_elem(node)?;
                            children.push(new_child);
                        },
                        _ => (),
                    }
                },
                SyntaxNodeElement::Leaf(leaf) => if leaf.value == ":" {
                    has_choices = true;
                },
            }
        }

        let mut choice = RuleChoice::new(RuleLookaheadKind::None, (1, 1), ASTReflection::Unreflectable, false, (1, 1), has_choices);
        choice.elem_containers = children;
        return Ok(choice);
    }

    fn to_rule_expr_elem(expr_node_list: &SyntaxNodeList) -> Result<RuleExpression, SyntaxParseError> {
        let expr_child_node = expr_node_list.get_node_list_child(0)?;

        let kind_and_value = match &expr_child_node.ast_reflection {
            ASTReflection::Reflectable(name) => {
                match name.as_str() {
                    "CharClass" => (RuleExpressionKind::ID, format!("[{}]", expr_child_node.to_string())),
                    "ID" => (RuleExpressionKind::ID, BlockParser::to_chain_id(expr_child_node.get_node_list_child(0)?)?),
                    "Str" => (RuleExpressionKind::String, expr_child_node.to_string()),
                    "Wildcard" => (RuleExpressionKind::Wildcard, ".".to_string()),
                    _ => return Err(SyntaxParseError::InternalErr(format!("unknown expression name '{}'", name))),
                }
            },
            _ => return Err(SyntaxParseError::InternalErr("invalid operation".to_string())),
        };

        let expr = RuleExpression::new(0, kind_and_value.0, RuleLookaheadKind::None, (1, 1), ASTReflection::Unreflectable, kind_and_value.1);
        return Ok(expr);
    }

    fn to_chain_id(chain_id_node: &SyntaxNodeList) -> Result<String, SyntaxParseError> {
        let mut ids = Vec::<String>::new();

        for chain_id_elem in &chain_id_node.filter_unreflectable_out() {
            ids.push(chain_id_elem.get_node_list()?.to_string());
        }

        return Ok(ids.join("."));
    }
}

struct FCPEGBlock {}

impl FCPEGBlock {
    pub fn get_block_map() -> BlockMap {
        return block_map!{
            "Main" => get_main_block,
            "Syntax" => get_syntax_block,
            "Symbol" => get_symbol_block,
            "Misc" => get_misc_block,
            "Block" => get_block_block,
            "Rule" => get_rule_block,
        };
    }

    fn get_main_block() -> Block {
        let start_cmd = start_cmd!("", "Syntax", "FCPEG");
        return block!("Main", vec![start_cmd]);
    }

    fn get_syntax_block() -> Block {
        let block_use = use_block!("Block");
        let symbol_use = use_block!("Symbol");

        // code: FCPEG <- Symbol.Space*# Symbol.LineEnd*# (Block.Block Symbol.LineEnd+#)* Symbol.LineEnd*# Symbol.Space*#,
        let fcpeg_rule = rule!{
            "FCPEG",
            choice!{
                vec![],
                expr!(ID, "Symbol.Space", "*", "#"),
                expr!(ID, "Symbol.LineEnd", "*", "#"),
                choice!{
                    vec!["*"],
                    choice!{
                        vec![],
                        expr!(ID, "Block.Block"),
                        expr!(ID, "Symbol.LineEnd", "+", "#"),
                    },
                },
                expr!(ID, "Symbol.LineEnd", "*", "#"),
                expr!(ID, "Symbol.Space", "*", "#"),
            },
        };

        return block!("Syntax", vec![block_use, symbol_use, fcpeg_rule]);
    }

    fn get_symbol_block() -> Block {
        // code: Space <- " ",
        let space_rule = rule!{
            "Space",
            choice!{
                vec![],
                expr!(String, " "),
            },
        };

        // code: LineEnd <- Space* "\n" Space*,
        let line_end_rule = rule!{
            "LineEnd",
            choice!{
                vec![],
                expr!(ID, "Space", "*"),
                expr!(String, "\n"),
                expr!(ID, "Space", "*"),
            },
        };

        return block!("Symbol", vec![space_rule, line_end_rule]);
    }

    fn get_misc_block() -> Block {
        // code: SingleID <- [a-zA-Z_] [a-zA-Z0-9_]*,
        let single_id_rule = rule!{
            "SingleID",
            choice!{
                vec![],
                expr!(CharClass, "[a-zA-Z_]"),
                expr!(CharClass, "[a-zA-Z0-9_]", "*"),
            },
        };

        // code: ChainID <- SingleID ("."# SingleID)*##,
        let chain_id_rule = rule!{
            "ChainID",
            choice!{
                vec![],
                expr!(ID, "SingleID"),
                choice!{
                    vec!["*", "##"],
                    choice!{
                        vec![],
                        expr!(String, ".", "#"),
                        expr!(ID, "SingleID"),
                    },
                },
            },
        };

        return block!("Misc", vec![single_id_rule, chain_id_rule]);
    }

    fn get_block_block() -> Block {
        let misc_use = use_block!("Misc");
        let rule_use = use_block!("Rule");
        let symbol_use = use_block!("Symbol");

        // code: Block <- "["# Symbol.Space*# Misc.SingleID Symbol.Space*# "]"# Symbol.Space*# "{"# Symbol.LineEnd+# (Cmd Symbol.LineEnd+#)* "}"#,
        let block_rule = rule!{
            "Block",
            choice!{
                vec![],
                expr!(String, "[", "#"),
                expr!(ID, "Symbol.Space", "*", "#"),
                expr!(ID, "Misc.SingleID"),
                expr!(ID, "Symbol.Space", "*", "#"),
                expr!(String, "]", "#"),
                expr!(ID, "Symbol.Space", "*", "#"),
                expr!(String, "{", "#"),
                expr!(ID, "Symbol.LineEnd", "+", "#"),
                choice!{
                    vec!["*"],
                    choice!{
                        vec![],
                        expr!(ID, "Cmd"),
                        expr!(ID, "Symbol.LineEnd", "+", "#"),
                    },
                },
                expr!(String, "}", "#"),
            },
        };

        // code: Cmd <- Comment : DefineCmd : StartCmd : UseCmd,
        let cmd_rule = rule!{
            "Cmd",
            choice!{
                vec![],
                choice!{
                    vec![":"],
                    choice!{
                        vec![],
                        expr!(ID, "Comment"),
                    },
                    choice!{
                        vec![],
                        expr!(ID, "DefineCmd"),
                    },
                    choice!{
                        vec![],
                        expr!(ID, "StartCmd"),
                    },
                    choice!{
                        vec![],
                        expr!(ID, "UseCmd"),
                    },
                },
            },
        };

        // code: Comment <- "%"# (!"," !Symbol.LineEnd .)* ","#,
        let comment_rule = rule!{
            "Comment",
            choice!{
                vec![],
                expr!(String, "%", "#"),
                choice!{
                    vec!["*"],
                    choice!{
                        vec![],
                        expr!(String, ",", "!"),
                        expr!(ID, "Symbol.LineEnd", "!"),
                        expr!(Wildcard, "."),
                    },
                },
                expr!(String, ",", "#"),
            },
        };

        // code: DefineCmd <- Misc.SingleID Symbol.Space*# "<-"# Symbol.Space*# Rule.PureChoice Symbol.Space*# ","#,
        let define_cmd_rule = rule!{
            "DefineCmd",
            choice!{
                vec![],
                expr!(ID, "Misc.SingleID"),
                expr!(ID, "Symbol.Space", "*", "#"),
                expr!(String, "<-", "#"),
                expr!(ID, "Symbol.Space", "*", "#"),
                expr!(ID, "Rule.PureChoice"),
                expr!(ID, "Symbol.Space", "*", "#"),
                expr!(String, ",", "#"),
            },
        };

        // code: StartCmd <- "+"# Symbol.Space*# "start"# Symbol.Space+# Misc.ChainID Symbol.Space*# ","#,
        let start_cmd_rule = rule!{
            "StartCmd",
            choice!{
                vec![],
                expr!(String, "+", "#"),
                expr!(ID, "Symbol.Space", "*", "#"),
                expr!(String, "start", "#"),
                expr!(ID, "Symbol.Space", "+", "#"),
                expr!(ID, "Misc.ChainID"),
                expr!(ID, "Symbol.Space", "*", "#"),
                expr!(String, ",", "#"),
            },
        };

        // code: UseCmd <- "+"# Symbol.Space*# "use"# Symbol.Space+# Misc.ChainID UseCmdBlockAlias? Symbol.Space*# ","#,
        let use_cmd_rule = rule!{
            "UseCmd",
            choice!{
                vec![],
                expr!(String, "+", "#"),
                expr!(ID, "Symbol.Space", "*", "#"),
                expr!(String, "use", "#"),
                expr!(ID, "Symbol.Space", "+", "#"),
                expr!(ID, "Misc.ChainID"),
                expr!(ID, "UseCmdBlockAlias", "?"),
                expr!(ID, "Symbol.Space", "*", "#"),
                expr!(String, ",", "#"),
            },
        };

        // code: UseCmdBlockAlias <- Symbol.Space+# "as" Symbol.Space+# Misc.SingleID,
        let use_cmd_block_alias_rule = rule!{
            "UseCmdBlockAlias",
            choice!{
                vec![],
                expr!(ID, "Symbol.Space", "+", "#"),
                expr!(String, "as", "#"),
                expr!(ID, "Symbol.Space", "+", "#"),
                expr!(ID, "Misc.SingleID"),
            },
        };

        return block!("Block", vec![misc_use, rule_use, symbol_use, block_rule, cmd_rule, comment_rule, define_cmd_rule, start_cmd_rule, use_cmd_rule, use_cmd_block_alias_rule]);
    }

    fn get_rule_block() -> Block {
        let misc_use = use_block!("Misc");
        let symbol_use = use_block!("Symbol");

        // code: PureChoice <- Seq (Symbol.Space# ":" Symbol.Space# Seq)*##,
        let pure_choice_rule = rule!{
            "PureChoice",
            choice!{
                vec![],
                expr!(ID, "Seq"),
                choice!{
                vec!["*", "##"],
                    expr!(ID, "Symbol.Space", "#"),
                    expr!(String, ":"),
                    expr!(ID, "Symbol.Space", "#"),
                    expr!(ID, "Seq"),
                },
            },
        };

        // code: Choice <- "("# PureChoice ")"#,
        let choice_rule = rule!{
            "Choice",
            choice!{
                vec![],
                expr!(String, "(", "#"),
                expr!(ID, "PureChoice"),
                expr!(String, ")", "#"),
            },
        };

        // code: Seq <- SeqElem (Symbol.Space+# SeqElem)*##,
        let seq_rule = rule!{
            "Seq",
            choice!{
                vec![],
                expr!(ID, "SeqElem"),
                choice!{
                    vec!["*", "##"],
                    choice!{
                        vec![],
                        choice!{
                            vec![],
                            expr!(ID, "Symbol.Space", "+", "#"),
                            expr!(ID, "SeqElem"),
                        },
                    },
                },
            },
        };

        // code: EscSeq <- "\\" ("\\" : "0" : "\"" : "n")##,
        let seq_elem_rule = rule!{
            "SeqElem",
            choice!{
                vec![],
                expr!(ID, "Lookahead", "?"),
                choice!{
                    vec!["##"],
                    choice!{
                        vec![":"],
                        choice!{
                            vec![],
                            expr!(ID, "Choice"),
                        },
                        choice!{
                            vec![],
                            expr!(ID, "Expr"),
                        },
                    },
                },
                expr!(ID, "Loop", "?"),
                expr!(ID, "RandomOrder", "?"),
                expr!(ID, "ASTReflection", "?"),
            },
        };

        // code: Expr <- ID : Str : CharClass : Wildcard,
        let expr_rule = rule!{
            "Expr",
            choice!{
                vec![],
                choice!{
                    vec![":"],
                    choice!{
                        vec![],
                        expr!(ID, "ID"),
                    },
                    choice!{
                        vec![],
                        expr!(ID, "Str"),
                    },
                    choice!{
                        vec![],
                        expr!(ID, "CharClass"),
                    },
                    choice!{
                        vec![],
                        expr!(ID, "Wildcard"),
                    },
                },
            },
        };

        // code: Lookahead <- "!" : "&",
        let lookahead_rule = rule!{
            "Lookahead",
            choice!{
                vec![],
                choice!{
                    vec![":"],
                    choice!{
                        vec![],
                        expr!(String, "!"),
                    },
                    choice!{
                        vec![],
                        expr!(String, "&"),
                    },
                },
            },
        };

        // code: Loop <- "?" : "*" : "+" : LoopRange,
        let loop_rule = rule!{
            "Loop",
            choice!{
                vec![],
                choice!{
                    vec![":"],
                    choice!{
                        vec![],
                        expr!(String, "?"),
                    },
                    choice!{
                        vec![],
                        expr!(String, "*"),
                    },
                    choice!{
                        vec![],
                        expr!(String, "+"),
                    },
                    choice!{
                        vec![],
                        expr!(ID, "LoopRange"),
                    },
                },
            },
        };

        // code: LoopRange <- "{"# Num? ","# Num? "}"#,
        let loop_range_rule = rule!{
            "LoopRange",
            choice!{
                vec![],
                expr!(String, "{", "#"),
                expr!(ID, "Num", "?"),
                expr!(String, ",", "#"),
                expr!(ID, "Num", "?"),
                expr!(String, "}", "#"),
            },
        };

        // expr: RandomOrder <- "^"# RandomOrderRange?,
        let random_order_rule = rule!{
            "RandomOrder",
            choice!{
                vec![],
                expr!(String, "^", "#"),
                expr!(String, "RandomOrderRange", "?"),
            },
        };

        // code: RandomOrderRange <- "["# Num? ","# Num? "]"#,
        let random_order_range_rule = rule!{
            "RandomOrderRange",
            choice!{
                vec![],
                expr!(String, "[", "#"),
                expr!(ID, "Num", "?"),
                expr!(String, "ID", "#"),
            },
        };

        // code: ASTReflection <- "#"# Misc.SingleID?,
        let ast_reflection_rule = rule!{
            "ASTReflection",
            choice!{
                vec![],
                expr!(String, "#", "#"),
                expr!(ID, "Misc.SingleID", "?"),
            },
        };

        // code: Num <- [0-9]+,
        let num_rule = rule!{
            "Num",
            choice!{
                vec![],
                expr!(CharClass, "[0-9]+", "+"),
            },
        };

        // code: ID <- Misc.ChainID,
        let id_rule = rule!{
            "ID",
            choice!{
                vec![],
                expr!(ID, "Misc.ChainID"),
            },
        };

        // code: EscSeq <- "\\" ("\\" : "0" : "\"" : "n"),
        let esc_seq_rule = rule!{
            "EscSeq",
            choice!{
                vec![],
                expr!(String, "\\"),
                choice!{
                    vec![],
                    choice!{
                        vec![":"],
                        choice!{
                            vec![],
                            expr!(String, "\\"),
                        },
                        choice!{
                            vec![],
                            expr!(String, "0"),
                        },
                        choice!{
                            vec![],
                            expr!(String, "\""),
                        },
                        choice!{
                            vec![],
                            expr!(String, "n"),
                        },
                    },
                },
            },
        };

        // code: Str <- "\""# ((EscSeq : !(("\\" : "\"")) .))+## "\""#,
        let str_rule = rule!{
            "Str",
            choice!{
                vec![],
                expr!(String, "\"", "#"),
                choice!{
                    vec!["+", "##"],
                    choice!{
                        vec![":"],
                        choice!{
                            vec![],
                            expr!(ID, "EscSeq"),
                        },
                        choice!{
                            vec![],
                            choice!{
                                vec!["!"],
                                choice!{
                                    vec![":"],
                                    choice!{
                                        vec![],
                                        expr!(String, "\\"),
                                    },
                                    choice!{
                                        vec![],
                                        expr!(String, "\""),
                                    },
                                },
                            },
                            expr!(Wildcard, "."),
                        },
                    },
                },
                expr!(String, "\"", "#"),
            },
        };

        // code: CharClass <- "["# (!"[" !"]" !Symbol.LineEnd (("\\[" : "\\]" : "\\\\" : .))##)+## "]"#,
        let char_class_rule = rule!{
            "CharClass",
            choice!{
                vec![],
                expr!(String, "[", "#"),
                choice!{
                    vec!["+", "##"],
                    expr!(String, "[", "!"),
                    expr!(String, "]", "!"),
                    expr!(ID, "Symbol.LineEnd", "!"),
                    choice!{
                        vec!["##"],
                        choice!{
                            vec![":"],
                            choice!{
                                vec![],
                                expr!(String, "\\["),
                            },
                            choice!{
                                vec![],
                                expr!(String, "\\]"),
                            },
                            choice!{
                                vec![],
                                expr!(String, "\\\\"),
                            },
                            choice!{
                                vec![],
                                expr!(Wildcard, "."),
                            },
                        },
                    },
                },
                expr!(String, "]", "#"),
            },
        };

        // code: Wildcard <- "."#,
        let wildcard_rule = rule!{
            "Wildcard",
            choice!{
                vec![],
                expr!(String, ".", "#"),
            },
        };

        return block!("Rule", vec![misc_use, symbol_use, pure_choice_rule, choice_rule, seq_rule, seq_elem_rule, expr_rule, lookahead_rule, loop_rule, loop_range_rule, random_order_rule, random_order_range_rule, ast_reflection_rule, num_rule, id_rule, esc_seq_rule, str_rule, char_class_rule, wildcard_rule]);
    }
}
