# SimpleDB

这是一个使用 Rust 编写的简易 SQL 数据库引擎。它实现了一个用于学习目的的完整数据库流程，包括 **词法分析 (Lexer)**、**语法分析 (Parser)**、**执行引擎 (Executor)** 以及 **存储引擎 (基于 JSON 的持久化)**。

## ✨ 功能特性

*   **数据持久化**：数据以 JSON 格式存储在本地文件中（默认为 `mydb.json`）。
*   **交互式 Shell**：支持类似 MySQL 的 REPL 命令行交互模式。
*   **基础数据类型**：支持 `INT` (整型) 和 `STRING` (字符串)。
*   **表管理**：创建 (`CREATE`) 和 删除 (`DROP`) 表。
*   **CRUD 操作**：支持 `INSERT`, `SELECT`, `UPDATE`, `DELETE`。
*   **条件查询**：支持简单的 `WHERE` 子句（支持 `=`, `>`, `<` 操作符）。
*   **多数据库切换**：支持运行时加载不同的数据库文件。

## 📦 依赖配置

在运行代码之前，请确保你的 `Cargo.toml` 文件中包含了以下依赖：

```toml
[dependencies]
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## 🚀 如何运行

1.  确保已安装 Rust 工具链。
2.  将代码保存为 `src/main.rs`。
3.  在项目根目录下运行：

```bash
cargo run
```

程序启动后会先运行一段演示代码，然后进入交互模式 (`SQL>`)。

## 📖 SQL 语法参考

以下是该数据库支持的指令示例。**注意：字符串必须使用单引号 `'` 包裹。**

### 1. 数据库与表操作 (DDL)

*   **查看所有表**
    ```sql
    SHOWTABLES
    ```

*   **创建表**
    ```sql
    -- 语法: CREATE TABLE <表名> (<列名> <类型>, ...)
    CREATE TABLE users (id INT, name STRING, age INT)
    ```

*   **删除表**
    ```sql
    DROP TABLE users
    ```

*   **加载/切换数据库文件**
    ```sql
    -- 加载指定路径的 JSON 数据库文件
    LOAD 'my_backup.json'
    ```

### 2. 数据操作 (DML)

*   **插入数据**
    ```sql
    -- 语法: INSERT INTO <表名> VALUES (<值1>, <值2>, ...)
    INSERT INTO users VALUES (1, 'Alice', 30)
    INSERT INTO users VALUES (2, 'Bob', 25)
    ```

*   **查询数据**
    ```sql
    -- 查询所有列
    SELECT * FROM users
    
    -- 查询指定列
    SELECT name, age FROM users
    
    -- 带条件的查询
    SELECT * FROM users WHERE age > 20
    ```

*   **更新数据**
    ```sql
    -- 语法: UPDATE <表名> SET <列名> = <值> WHERE <条件>
    UPDATE users SET age = 31 WHERE name = 'Alice'
    ```

*   **删除数据**
    ```sql
    -- 语法: DELETE FROM <表名> WHERE <条件>
    DELETE FROM users WHERE id = 2
    ```

### 3. 退出程序

在提示符下输入：
```text
exit
```
## 🛠️ 系统架构简述

1.  **Storage Engine**: 使用 `serde_json` 将内存中的 `HashMap<String, Table>` 结构序列化为磁盘上的 JSON 文件。
2.  **Lexer**: 手写词法分析器，将原始 SQL 字符串转换为 `Token` 流。
3.  **Parser**: 基于递归下降算法，将 Token 流解析为抽象语法树。
4.  **Executor**: 遍历 AST，对内存中的数据结构进行增删改查，并在操作变更时自动触发 `save` 方法将数据写入磁盘。

## ⚠️ 目前限制

*   `WHERE` 子句仅支持单个条件（不支持 `AND` / `OR`）。
*   比较操作符仅支持 `=`, `>`, `<`。
*   不支持主键约束、外键或索引。
*   不支持 `JOIN` 操作。
*   SQL 关键字不区分大小写，但标识符（表名、列名）区分大小写。
