use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Write};
use std::fs::{File};
use std::iter::Peekable;
use std::str::Chars;

// ==========================================
// 1. Storage Engine (存储引擎 & 数据模型)
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
enum DataType {
    Integer(i64),
    String(String),
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Integer(v) => write!(f, "{}", v),
            DataType::String(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ColumnDef {
    name: String,
    col_type: String, // "INT" or "STRING"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Table {
    name: String,
    columns: Vec<ColumnDef>,
    rows: Vec<Vec<DataType>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Database {
    tables: HashMap<String, Table>,
}

impl Database {
    fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    // 持久化：加载
    fn load(path: &str) -> Result<Self> {
        if std::path::Path::new(path).exists() {
            let file = File::open(path)?;
            let db = serde_json::from_reader(file)?;
            Ok(db)
        } else {
            Ok(Self::new())
        }
    }

    // 持久化：保存
    fn save(&self, path: &str) -> Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}

// ==========================================
// 2. Lexer 
// ==========================================

#[derive(Debug, Clone, PartialEq)]
enum Token {
    // Keywords
    ShowTables, Select, From, Where, Insert, Into, Values, Update, Set, Delete, Create, Table, Drop, IntType, StringType, Load, 
    // Symbols
    Star, Eq, Gt, Lt, Comma, LParen, RParen, SemiColon,
    // Values
    Identifier(String),
    Number(i64),
    StringLiteral(String),
    EOF,
}

struct Lexer<'a> {
    iter: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { iter: input.chars().peekable() }
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        
        match self.iter.next() {
            None => Token::EOF,
            Some(c) => match c {
                '*' => Token::Star,
                '=' => Token::Eq,
                '>' => Token::Gt,
                '<' => Token::Lt,
                ',' => Token::Comma,
                '(' => Token::LParen,
                ')' => Token::RParen,
                ';' => Token::SemiColon,
                '\'' => self.read_string(),
                c if c.is_digit(10) => self.read_number(c),
                c if c.is_alphabetic() => self.read_identifier(c),
                _ => panic!("Unexpected character: {}", c),
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.iter.peek() {
            if c.is_whitespace() {
                self.iter.next();
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self) -> Token {
        let mut s = String::new();
        while let Some(&c) = self.iter.peek() {
            if c == '\'' {
                self.iter.next();
                return Token::StringLiteral(s);
            }
            s.push(c);
            self.iter.next();
        }
        panic!("Unterminated string literal");
    }

    fn read_number(&mut self, start: char) -> Token {
        let mut s = start.to_string();
        while let Some(&c) = self.iter.peek() {
            if c.is_digit(10) {
                s.push(c);
                self.iter.next();
            } else {
                break;
            }
        }
        Token::Number(s.parse().unwrap())
    }

    fn read_identifier(&mut self, start: char) -> Token {
        let mut s = start.to_string();
        while let Some(&c) = self.iter.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.iter.next();
            } else {
                break;
            }
        }
        match s.to_uppercase().as_str() {
            "SHOWTABLES" => Token::ShowTables, 
            "SELECT" => Token::Select,
            "FROM" => Token::From,
            "WHERE" => Token::Where,
            "INSERT" => Token::Insert,
            "INTO" => Token::Into,
            "VALUES" => Token::Values,
            "UPDATE" => Token::Update,
            "SET" => Token::Set,
            "DELETE" => Token::Delete,
            "CREATE" => Token::Create,
            "TABLE" => Token::Table,
            "DROP" => Token::Drop,
            "INT" => Token::IntType,
            "STRING" => Token::StringType,
            "LOAD" => Token::Load,
            _ => Token::Identifier(s),
        }
    }
    
    // 一次性获取所有Token
    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            if token == Token::EOF {
                break;
            }
            tokens.push(token);
        }
        tokens
    }
}

// ==========================================
// 3. Parser (语法分析器 & AST)
// ==========================================

// --- AST Definitions ---
#[derive(Debug)]
enum Stmt {
    ShowTables,
    Select { table: String, columns: Vec<String>, where_expr: Option<Expr> },
    Insert { table: String, values: Vec<DataType> },
    Update { table: String, assignments: Vec<(String, DataType)>, where_expr: Option<Expr> },
    Delete { table: String, where_expr: Option<Expr> },
    CreateTable { name: String, columns: Vec<ColumnDef> },
    DropTable { name: String },
    LoadDatabase { path: String },
}

#[derive(Debug, Clone)]
enum Expr {
    BinaryOp { left: String, op: Op, right: DataType }, // 简化：仅支持 列 OP 常量
}

#[derive(Debug, Clone)]
enum Op { Eq, Gt, Lt }

// --- Parser Implementation ---
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) -> &Token {
        let t = self.tokens.get(self.pos).unwrap_or(&Token::EOF);
        self.pos += 1;
        t
    }

    fn consume(&mut self, expected: Token) -> Result<()> {
        if self.current() == &expected {
            self.advance();
            Ok(())
        } else {
            Err(anyhow!("Expected {:?}, found {:?}", expected, self.current()))
        }
    }

    fn parse(&mut self) -> Result<Stmt> {
        match self.current() {
            Token::Select => self.parse_select(),
            Token::Insert => self.parse_insert(),
            Token::Update => self.parse_update(),
            Token::Delete => self.parse_delete(),
            Token::Create => self.parse_create(),
            Token::Drop => self.parse_drop(),
            Token::Load => self.parse_load(),
            Token::ShowTables => {
                self.advance(); 
                Ok(Stmt::ShowTables)
            }
            _ => Err(anyhow!("Unknown statement start: {:?}", self.current())),
        }
    }

    fn parse_select(&mut self) -> Result<Stmt> {
        self.consume(Token::Select)?;
        let mut columns = Vec::new();
        if self.current() == &Token::Star {
            self.advance();
        } else {
            loop {
                if let Token::Identifier(name) = self.advance() {
                    columns.push(name.clone());
                } else {
                    return Err(anyhow!("Expected column name"));
                }
                if self.current() == &Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.consume(Token::From)?;
        let table = match self.advance() {
            Token::Identifier(name) => name.clone(),
            _ => return Err(anyhow!("Expected table name")),
        };
        
        let where_expr = self.parse_where_clause()?;
        Ok(Stmt::Select { table, columns, where_expr })
    }

    fn parse_insert(&mut self) -> Result<Stmt> {
        self.consume(Token::Insert)?;
        self.consume(Token::Into)?;
        let table = match self.advance() {
            Token::Identifier(name) => name.clone(),
            _ => return Err(anyhow!("Expected table name")),
        };
        self.consume(Token::Values)?;
        self.consume(Token::LParen)?;
        
        let mut values = Vec::new();
        loop {
            values.push(self.parse_value()?);
            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        self.consume(Token::RParen)?;
        Ok(Stmt::Insert { table, values })
    }

    fn parse_update(&mut self) -> Result<Stmt> {
        self.consume(Token::Update)?;
        let table = match self.advance() {
            Token::Identifier(name) => name.clone(),
            _ => return Err(anyhow!("Expected table name")),
        };
        self.consume(Token::Set)?;
        
        let mut assignments = Vec::new();
        loop {
            let col = match self.advance() {
                Token::Identifier(name) => name.clone(),
                _ => return Err(anyhow!("Expected column name")),
            };
            self.consume(Token::Eq)?;
            let val = self.parse_value()?;
            assignments.push((col, val));

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        let where_expr = self.parse_where_clause()?;
        Ok(Stmt::Update { table, assignments, where_expr })
    }

    fn parse_delete(&mut self) -> Result<Stmt> {
        self.consume(Token::Delete)?;
        self.consume(Token::From)?;
        let table = match self.advance() {
            Token::Identifier(name) => name.clone(),
            _ => return Err(anyhow!("Expected table name")),
        };
        let where_expr = self.parse_where_clause()?;
        Ok(Stmt::Delete { table, where_expr })
    }

    fn parse_create(&mut self) -> Result<Stmt> {
        self.consume(Token::Create)?;
        self.consume(Token::Table)?;
        let name = match self.advance() {
            Token::Identifier(n) => n.clone(),
            _ => return Err(anyhow!("Expected table name")),
        };
        self.consume(Token::LParen)?;
        
        let mut columns = Vec::new();
        loop {
            let col_name = match self.advance() {
                Token::Identifier(n) => n.clone(),
                _ => return Err(anyhow!("Expected column name")),
            };
            let col_type = match self.advance() {
                Token::IntType => "INT".to_string(),
                Token::StringType => "STRING".to_string(),
                _ => return Err(anyhow!("Expected type INT or STRING")),
            };
            columns.push(ColumnDef { name: col_name, col_type });

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        self.consume(Token::RParen)?;
        Ok(Stmt::CreateTable { name, columns })
    }

    fn parse_drop(&mut self) -> Result<Stmt> {
        self.consume(Token::Drop)?;
        self.consume(Token::Table)?;
        let name = match self.advance() {
            Token::Identifier(n) => n.clone(),
            _ => return Err(anyhow!("Expected table name")),
        };
        Ok(Stmt::DropTable { name })
    }

    fn parse_load(&mut self) -> Result<Stmt> {
        self.consume(Token::Load)?;
        // 期望下一个是字符串字面量，作为文件路径
        let path = match self.advance() {
            Token::StringLiteral(s) => s.clone(),
            _ => return Err(anyhow!("Expected string literal for database path")),
        };
        Ok(Stmt::LoadDatabase { path })
    }

    // 辅助解析：WHERE 子句 (简化版: 仅支持 col op val)
    fn parse_where_clause(&mut self) -> Result<Option<Expr>> {
        if self.current() == &Token::Where {
            self.advance();
            let left = match self.advance() {
                Token::Identifier(n) => n.clone(),
                _ => return Err(anyhow!("Expected column in WHERE")),
            };
            let op = match self.advance() {
                Token::Eq => Op::Eq,
                Token::Gt => Op::Gt,
                Token::Lt => Op::Lt,
                _ => return Err(anyhow!("Expected =, >, <")),
            };
            let right = self.parse_value()?;
            Ok(Some(Expr::BinaryOp { left, op, right }))
        } else {
            Ok(None)
        }
    }

    fn parse_value(&mut self) -> Result<DataType> {
        match self.advance() {
            Token::Number(n) => Ok(DataType::Integer(*n)),
            Token::StringLiteral(s) => Ok(DataType::String(s.clone())),
            _ => Err(anyhow!("Expected value")),
        }
    }
}

// ==========================================
// 4. Executor (执行引擎)
// ==========================================

struct Executor {
    db: Database,
    filepath: String,
}

impl Executor {
    fn new(filepath: &str) -> Result<Self> {
        let db = Database::load(filepath)?;
        Ok(Self { db, filepath: filepath.to_string() })
    }

    fn execute(&mut self, sql: &str) -> Result<String> {
        let mut lexer = Lexer::new(sql);
        let tokens = lexer.tokenize();
        if tokens.is_empty() { return Ok("".to_string()); }

        let mut parser = Parser::new(tokens);
        let stmt = parser.parse()?;

        match stmt {
            Stmt::ShowTables => {
                let mut tables: Vec<_> = self.db.tables.keys().collect();
                tables.sort();
                if tables.is_empty() {
                    Ok("No tables found".to_string())
                } else {
                    Ok(format!("Tables:\n{}", tables.into_iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n")))
                }
            }
            Stmt::CreateTable { name, columns } => {
                if self.db.tables.contains_key(&name) {
                    return Err(anyhow!("Table already exists"));
                }
                let table = Table { name: name.clone(), columns, rows: Vec::new() };
                self.db.tables.insert(name, table);
                self.save()?;
                Ok("Table created".to_string())
            }
            Stmt::DropTable { name } => {
                self.db.tables.remove(&name).ok_or(anyhow!("Table not found"))?;
                self.save()?;
                Ok("Table dropped".to_string())
            }
            Stmt::Insert { table, values } => {
                let t = self.db.tables.get_mut(&table).ok_or(anyhow!("Table not found"))?;
                if values.len() != t.columns.len() {
                    return Err(anyhow!("Column count mismatch"));
                }
                // 类型检查略... 假设用户输入正确
                t.rows.push(values);
                self.save()?;
                Ok("Rows inserted".to_string())
            }
            Stmt::Select { table, columns, where_expr } => {
                let t = self.db.tables.get(&table).ok_or(anyhow!("Table not found"))?;
                let mut result = String::new();
                
                // Print Header
                let col_indices: Vec<usize> = if columns.is_empty() {
                    (0..t.columns.len()).collect()
                } else {
                    columns.iter().map(|c| t.columns.iter().position(|def| &def.name == c).expect("Col not found")).collect()
                };
                
                for idx in &col_indices {
                    result.push_str(&format!("{}\t", t.columns[*idx].name));
                }
                result.push('\n');

                // Print Rows
                for row in &t.rows {
                    if let Some(expr) = &where_expr {
                        if !Self::evaluate_where(expr, row, &t.columns) {
                            continue;
                        }
                    }
                    for idx in &col_indices {
                        result.push_str(&format!("{}\t", row[*idx]));
                    }
                    result.push('\n');
                }
                Ok(result)
            }
            Stmt::Update { table, assignments, where_expr } => {
                let t = self.db.tables.get_mut(&table).ok_or(anyhow!("Table not found"))?;
                let mut count = 0;
                
                // 预计算列索引
                let mut updates = Vec::new();
                for (col_name, val) in assignments {
                    let idx = t.columns.iter().position(|c| c.name == col_name).ok_or(anyhow!("Col not found"))?;
                    updates.push((idx, val));
                }

                for row in t.rows.iter_mut() {
                    let match_cond = match &where_expr {
                        Some(expr) => Self::evaluate_where(expr, row, &t.columns),
                        None => true,
                    };
                    if match_cond {
                        for (idx, val) in &updates {
                            row[*idx] = val.clone();
                        }
                        count += 1;
                    }
                }
                self.save()?;
                Ok(format!("Updated {} rows", count))
            }
            Stmt::Delete { table, where_expr } => {
                let deleted_count = {
                    let t = self.db.tables.get_mut(&table).ok_or(anyhow!("Table not found"))?;
                    let original_len = t.rows.len();
                    
                    let columns = t.columns.clone();
                    t.rows.retain(|row| {
                         match &where_expr {
                            Some(expr) => !Self::evaluate_where(expr, row, &columns),
                            None => false,
                        }
                    });
                    
                    // 计算删除行数并返回
                    original_len - t.rows.len()
                }; 
                self.save()?;
                Ok(format!("Deleted {} rows", deleted_count))
            }
            Stmt::LoadDatabase { path } => {
                self.load(&path) // 调用最开始写的 load 函数
            }
        }
    }

    fn evaluate_where(expr: &Expr, row: &Vec<DataType>, columns: &Vec<ColumnDef>) -> bool {
        match expr {
            Expr::BinaryOp { left, op, right } => {
                let col_idx = columns.iter().position(|c| &c.name == left).expect("Col not found in where");
                let cell_val = &row[col_idx];
                match op {
                    Op::Eq => cell_val == right,
                    Op::Gt => cell_val > right,
                    Op::Lt => cell_val < right,
                }
            }
        }
    }

    fn save(&self) -> Result<()> {
        self.db.save(&self.filepath)
    }

    pub fn load(&mut self, path: &str) -> Result<String> {
        let new_db = Database::load(path)?;
        self.db = new_db;
        self.filepath = path.to_string();
        Ok(format!("Database loaded from '{}'", path))
    }
}

// ==========================================
// 5. Main (入口)
// ==========================================

fn main() -> Result<()> {
    let mut exec = Executor::new("mydb.json")?;
    
    println!("SimpleDB v1.0 - Type 'exit' to quit");
    
    // 简单演示模式
    let demo_sqls = vec![
        "LOAD 'mydb.json'",
        "SHOWTABLES",
        "DROP TABLE users",
        "CREATE TABLE users (id INT, name STRING, age INT)",
        "INSERT INTO users VALUES (1, 'Alice', 30)",
        "INSERT INTO users VALUES (2, 'Bob', 25)",
        "INSERT INTO users VALUES (3, 'Charlie', 35)",
        "SELECT * FROM users",
        "UPDATE users SET age = 99 WHERE id = 1",
        "DELETE FROM users WHERE age < 30",
        "SELECT name, age FROM users",
        "LOAD 'another_db.json'",
        "SHOWTABLES",
        "DROP TABLE users",
        "CREATE TABLE users (id INT, name STRING, age INT)",
        "SELECT * FROM users",
    ];

    for sql in demo_sqls {
        println!("SQL> {}", sql);
        match exec.execute(sql) {
            Ok(res) => println!("{}", res),
            Err(e) => println!("Error: {}", e), 
        }
        println!("----------------");
    }
        println!("Interactive mode started. Type 'exit' to quit.");

    loop {
        // 1. 打印提示符 "SQL> "
        print!("SQL> ");
        // 刷新缓冲区，确保提示符立即显示，而不是等换行才显示
        io::stdout().flush()?; 

        // 2. 读取用户输入
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        // 3. 去除首尾空白字符 (包括换行符)
        let command = input.trim();

        // 4. 处理特殊情况
        if command.is_empty() {
            continue;
        }

        if command.eq_ignore_ascii_case("exit") || command.eq_ignore_ascii_case("quit") {
            println!("Bye!");
            break;
        }

        // 5. 执行 SQL 并打印结果
        match exec.execute(command) {
            Ok(res) => println!("{}", res),
            Err(e) => println!("Error: {}", e),
        }
        println!("----------------");
    }

    Ok(())
}