# Action Language

Action 是一门静态类型的多范式编程语言，编译器用 Rust 编写，基于 LLVM 后端，支持 JIT 即时编译与原生代码生成。

## 特性

- **可空类型** — Kotlin 风格 `T?` 空安全，方法/字段/索引自动短路传播，`or {}` 默认值，智能转换
- **泛型** — 泛型函数 `fun <T> id(x: T): T`，泛型枚举 `Option[T]`
- **静态类型系统** — 结构化类型，类型推断，类型别名
- **模式匹配** — 穷尽性 `when` 表达式，支持枚举/结构体解构，守卫（guard）与或模式
- **一等函数** — Lambda 表达式，隐式 `it` 参数，闭包捕获
- **集合类型** — List, Set, Map 及丰富的方法链
- **扩展方法** — 为已有类型添加方法
- **字符串插值** — `"Hello, ${name}!"`
- **协程与流** — 轻量级 Task/Stream，支持异步通信
- **HTTP 客户端** — 内建 HTTP 请求支持
- **JSON 支持** — 解析、序列化、遍历
- **LSP** — 内置 Language Server Protocol 支持
- **FFI** — `external` 关键字，支持调用 C 函数
- **跨平台** — LLVM 后端支持 Linux x64/ARM64、Windows x64、WASM

## 快速开始

### 从源码构建

需要 Rust 工具链和 LLVM 21+：

```bash
git clone https://github.com/TetraploidHuman/Action.git
cd Action
cargo build --release
```

### Hello World

```action
fun main() {
    println("Hello, World!")
}
```

```bash
action run hello.at
```

## 语言指南

### 变量

```action
val x = 42              // 不可变绑定
var y = 0               // 可变绑定
val name: String = "Action"  // 带类型标注
val lazy big = heavyComputation()  // 惰性初始化

y += 1                  // 复合赋值: += -= *= /= %=
y = y + 1               // 普通赋值
```

### 基本类型

```action
val i: Int = 42
val f: Float = 3.14
val b: Bool = true           // true / false
val s: String = "hello"
val c: Char = 'A'
```

### 运算符

```action
// 算术
a + b    a - b    a * b    a / b    a % b
a ** b   // 幂运算

// 比较（返回 Bool）
a == b   a != b   a < b    a > b    a <= b   a >= b

// 逻辑（短路求值）
a and b    a or b    not a

// 位运算
a & b    a | b    a ^ b    a << b    a >> b    ~a
// 可用作前缀运算符: a & b 等价于 bitAnd(a, b)
```

### 字符串

```action
val s = "hello"

s.trim()                    // "hi"（去除两端空白）
s.toUpper()                 // "HELLO"
s.toLower()                 // "hello"
s.len()                     // 5
s.substring(0, 2)           // "he"
s.split(",")                // List["a", "b", "c"]
s.startsWith("he")          // true
s.endsWith("lo")            // true
s.contains("ell")           // true
s.charAt(1)                 // Char 'e'
s.charCode(0)               // Int 104
s.chars()                   // List['h', 'e', 'l', 'l', 'o']
s.indexOf("el")             // Int? = 1（可空）
s.replace("foo", "bar")     // 替换
s.repeat(3)                 // 重复
s.trimStart()               // 去除左端空白
s.trimEnd()                 // 去除右端空白
s.join(List["x", "y"])      // List[String] → String: "hello"
"42".toInt()                // Int? = 42（可空）
"3.14".toFloat()            // Float? = 3.14（可空）

// 字符串插值
val name = "World"
val msg = "Hello, ${name}!"
```

### 条件表达式

```action
// when 作为表达式（单行）
val s = when x > 0 { "positive" else "non-positive" }

// when 作为表达式（多臂）
val desc = when x {
    0 -> "zero"
    1 -> "one"
    else -> "other"
}

// 值匹配 when（可对可空值进行）
val result = when maybe(x) {
    null -> "none"
    42   -> "answer"
    else -> "other"
}

// 带守卫的模式匹配
val r = when x {
    n if n > 0 -> "positive"
    n if n < 0 -> "negative"
    else       -> "zero"
}
```

### 函数

```action
// 单表达式函数
fun add(a: Int, b: Int): Int { a + b }

// 多语句函数
fun greet(name: String) {
    println("Hello, " + name)
}

// 递归函数
fun fib(n: Int): Int {
    when n <= 1 { n else fib(n - 1) + fib(n - 2) }
}

// 尾递归优化
fun sum(n: Int, acc: Int): Int {
    when n <= 0 { acc else sum(n - 1, acc + n) }
}
```

### Lambda 表达式

```action
val double = { x -> x * 2 }       // 显式参数
val triple = { it * 3 }           // 隐式 it 参数

// 高阶函数
val nums = List[1, 2, 3, 4, 5]
val doubled = nums.map { it * 2 }
val evens = nums.filter { it % 2 == 0 }
val sum = nums.fold(0) { acc, x -> acc + x }
val all_positive = nums.all { it > 0 }
val has_even = nums.any { it % 2 == 0 }
```

### 泛型

```action
// 泛型函数
fun <T> identity(x: T): T { x }

fun <T> pickFirst(a: T, b: T): T { a }

// 泛型枚举
enum Option[T] {
    Some(T),
    None
}

// 使用泛型函数（自动推断类型参数）
val x = identity(42)        // Int
val y = identity("hello")   // String
val z = identity(3.14)      // Float
```

### 可空类型

```action
// T? 表示可空类型，null 表示空值
val name: String? = "Alice"
val empty: String? = null

// 对可空接收者的方法调用自动短路（接收者为 null 时直接返回 null）
val upper = name.toUpper()         // String? = "ALICE"
val none = empty.toUpper()         // String? = null

// 字段访问自动短路
val city = user.address.city       // String? — 任一环节为 null 则整体为 null

// 索引操作自动短路
val item = list[0]                 // 若 list 为 null，返回 null

// or { } 提供默认值
val display = name or { "Guest" }
val cityName = user?.address?.city or { "Unknown" }

// or { } 允许 return 提前终止，实现错误传播
fun process(): String? {
    val x = maybe() or { return null }
    val y = another(x) or { return null }
    toUpper(x + y)
}

// 智能转换 — 空判断后自动转换为非空类型
if name != null {
    println(name.toUpper())    // name 自动提升为 String
}

// 智能转换也适用于 when
when name {
    null -> println("got null")
    else -> println(name.len()) // name 提升为 String
}
```

### 枚举

```action
enum Color {
    Red,
    Green,
    Blue
}

enum Shape {
    Circle(Int),        // 半径
    Rectangle(Int, Int) // 宽, 高
}

// 模式匹配解构
val area = when shape {
    Circle(r)  -> 3.14 * r * r
    Rectangle(w, h) -> w * h
}

// 或模式
val c = when color {
    Red | Green -> "warm"
    Blue        -> "cool"
}
```

### 结构体

```action
type Point = { x: Int, y: Int }

// 字面量构造（字段顺序无关）
val p = { x = 10, y = 20 }

// 简写构造
val x = 10; val y = 20
val p2 = { x, y }

// 字段访问
val px = p.x

// 解构
val {x, y} = p
val {x as px, y as py} = p  // 重命名
```

### 扩展方法

```action
// 为现有类型添加方法
extension Int {
    fun square(): Int { this * this }
    fun isEven(): Bool { this % 2 == 0 }
}

5.square()     // 25
6.isEven()     // true
```

### 类型别名

```action
type UserId = Int
type Person = { id: UserId, name: String }
```

### For 循环

```action
// 遍历集合
for item in List[1, 2, 3] {
    println(item)
}

// 遍历范围（含边界）
for i in 1..5 {
    print(i)
}

// 遍历范围（不含边界）
for i in 1..<5 {
    print(i)       // 1, 2, 3, 4
}

// 遍历集合（带下标）
for index, item in list {
    println("${index}: ${item}")
}

// 条件循环
var i = 0
for i < 10 {
    i = i + 1
}

// for 作为表达式（收集结果）
val doubled = for x in list { x * 2 }
val evens = for x in list if x % 2 == 0 { x }
```

### 集合

```action
// List — 构建与链式操作
val list = List[1, 2, 3, 4, 5]

list.len()              // 5
list[0]                 // 1（索引访问）
list.head()             // 1
list.last()             // 5
list.tail()             // List[2, 3, 4, 5]
list.contains(3)        // true
list.indexOf(3)         // Int? = 2
list.reverse()          // List[5, 4, 3, 2, 1]
list.take(2)            // List[1, 2]
list.drop(2)            // List[3, 4, 5]
list.append(6)          // List[1, 2, 3, 4, 5, 6]
list.map { it * 2 }     // List[2, 4, 6, 8, 10]
list.filter { it % 2 == 0 } // List[2, 4]
list.fold(0) { acc, x -> acc + x }  // 15
list.isEmpty()          // true/false
list.sorted()           // List[1, 2, 3, 4, 5]
list.unique()           // 去重
list.find { it == 3 }   // Int? = 3
list.sum()              // 15
list.product()          // 120
list.zip(List["a", "b", "c"])  // List[{1, "a"}, {2, "b"}, {3, "c"}]

// Set
val s = Set[1, 2, 3, 3, 2]  // Set[1, 2, 3]
s.contains(2)            // true

// Map
val m = Map["a": 1, "b": 2]
m.get("a")               // Int? = 1
m.contains("a")          // true
m.insert("c", 3)         // Map["a": 1, "b": 2, "c": 3]
m.remove("a")            // Map["b": 2]
m.len()                  // 2
m.keys()                 // List["a", "b"]
m.values()               // List[1, 2]
```

### 文件 I/O

```action
// API 风格与版本有关，具体用法参考示例
appendFile("/tmp/log.txt", "log entry\n")
```

### HTTP 请求

```action
val resp = httpRequest(
    "GET",
    "https://httpbin.org/get",
    "Accept: application/json",
    ""
)
println(resp)
// 返回 "状态码\n响应体"
```

### JSON 支持

```action
// 解析 JSON 字符串
val json = action_json_parse("{\"a\": 1, \"b\": 2}")

// 获取字段值
val a = action_json_get(json, "a")
val aVal = action_json_as_float(a)  // 1.0

// 类型检查
val t = action_json_type(json)     // 5 = object

// 序列化
val str = action_json_stringify(json)  // "{\"a\":1,\"b\":2}"

// 类型常量: 0=null, 1=bool, 2=number, 3=string, 4=array, 5=object
```

### 类型转换

```action
val f = toFloat(42)              // Int → Float: 42.0
val i = toInt(3.14)              // Float → Int: 3
"42".toInt()                     // Int? = 42（可空，解析失败为 null）
"3.14".toFloat()                 // Float? = 3.14
```

### 数学函数

```action
abs(-5)          // 5
min(3, 7)        // 3
max(3, 7)        // 7
sqrt(16.0)       // 4.0
pow(2.0, 10.0)   // 1024.0
sin(x)           // 正弦
cos(x)           // 余弦
tan(x)           // 正切
floor(3.7)       // 3.0
ceil(3.2)        // 4.0
round(3.5)       // 4.0
log(10.0)        // 自然对数
log2(8.0)        // 3.0
log10(100.0)     // 2.0
exp(1.0)         // e
cbrt(27.0)       // 3.0（立方根）
gcd(48, 18)      // 6
```

### 协程与流

```action
// 创建流
val (rx, tx) = stream()

// 启动异步任务
val task = launch {
    send(tx, 42)
    println("sent 42")
}

val msg = recv(rx)          // 接收消息
val done = is_closed(rx)    // 检查流是否关闭
```

### 模块系统

```action
// 导入整个模块（通过模块名访问其成员）
import math

// 选择性导入（直接使用导入的项）
import math.{sin, cos}

// 带别名的完整导入
import math as m
m.sin(3.14)

// 导出
export fun helper() { 42 }
```

### FFI

```action
// 声明外部 C 函数
external fun printf(format: String, ...): Int

// 声明外部类型
external type FileHandle

// unsafe 块用于调用外部函数
unsafe {
    printf("hello %d\n", 42)
}
```

## 完整方法速查

### String 方法
| 方法 | 返回 | 说明 |
|------|------|------|
| `.len()` / `.length()` | Int | 长度 |
| `.toUpper()` | String | 转大写 |
| `.toLower()` | String | 转小写 |
| `.trim()` | String | 去除两端空白 |
| `.trimStart()` / `.trimEnd()` | String | 去除左/右端空白 |
| `.split(delim)` | List[String] | 分割 |
| `.join(list)` | String | 连接列表 |
| `.substring(from, to)` | String | 取子串 |
| `.startsWith(prefix)` | Bool | 前缀匹配 |
| `.endsWith(suffix)` | Bool | 后缀匹配 |
| `.contains(substr)` | Bool | 包含检查 |
| `.replace(old, new)` | String | 替换子串 |
| `.repeat(n)` | String | 重复拼接 |
| `.charAt(idx)` | Char | 取字符 |
| `.charCode(idx)` | Int | 取 ASCII 码 |
| `.chars()` | List[Char] | 转字符列表 |
| `.indexOf(sub)` | Int? | 查找子串位置 |
| `.toInt()` | Int? | 解析为整数 |
| `.toFloat()` | Float? | 解析为浮点数 |

### List 方法
| 方法 | 返回 | 说明 |
|------|------|------|
| `.len()` | Int | 长度 |
| `.head()` | T? | 首元素 |
| `.last()` | T? | 尾元素 |
| `.tail()` | List[T]? | 除首元素外的子列表 |
| `.init()` | List[T]? | 除尾元素外的子列表 |
| `.get(idx)` | T? | 索引访问 |
| `.contains(elem)` | Bool | 包含检查 |
| `.indexOf(elem)` | Int? | 查找索引 |
| `.append(elem)` | List[T] | 追加 |
| `.reverse()` | List[T] | 反转 |
| `.take(n)` | List[T] | 取前 n 个 |
| `.drop(n)` | List[T] | 去掉前 n 个 |
| `.sorted()` | List[T] | 排序 |
| `.unique()` | List[T] | 去重 |
| `.isEmpty()` | Bool | 判空 |
| `.map(fn)` | List[U] | 映射 |
| `.filter(fn)` | List[T] | 过滤 |
| `.fold(init, fn)` | U | 折叠 |
| `.any(fn)` | Bool | 任一满足 |
| `.all(fn)` | Bool | 全部满足 |
| `.find(fn)` | T? | 查找 |
| `.sum()` | Int/Float | 求和 |
| `.product()` | Int/Float | 求积 |
| `.zip(other)` | List[{T, U}] | 压缩 |
| `.flatten()` | List[T] | 展平嵌套列表 |

### Map 方法
| 方法 | 返回 | 说明 |
|------|------|------|
| `.len()` | Int | 大小 |
| `.get(key)` | V? | 取值 |
| `.contains(key)` | Bool | 键存在 |
| `.insert(key, val)` | Map | 插入/更新 |
| `.remove(key)` | Map | 删除键 |
| `.keys()` | List[K] | 所有键 |
| `.values()` | List[V] | 所有值 |

### Set 方法
| 方法 | 返回 | 说明 |
|------|------|------|
| `.len()` | Int | 大小 |
| `.contains(elem)` | Bool | 包含检查 |

## 命令行

```bash
action run file.at                    # 编译并运行（JIT）
action build file.at                  # 编译为 LLVM IR
action build file.at -o prog          # 编译为可执行文件
action check file.at                 # 仅类型检查，不运行

action run file.at --check           # 类型检查 + JIT
action run file.at -O 3              # 优化等级 0-3
action run file.at --emit ir         # 输出 LLVM IR
action build file.at --emit asm      # 输出汇编
action build file.at --emit obj      # 输出目标文件
action build file.at --emit exe      # 链接为可执行文件
action run file.at --target wasm     # 交叉编译到 WASM
action build file.at --target linux-arm64  # 交叉编译到 ARM64

# 启动 LSP 服务器
action lsp
```

### 目标平台

| 值 | 说明 |
|------|------|
| `native` | 当前平台（默认） |
| `linux-x64` | Linux x86_64 |
| `linux-arm64` | Linux ARM64 |
| `windows-x64` | Windows x86_64 |
| `wasm` | WebAssembly |

### `--emit` 格式

| 值 | 说明 |
|------|------|
| `ir` | LLVM IR（打印到 stdout） |
| `bc` | LLVM 位码 |
| `asm` / `s` | 汇编 |
| `obj` / `o` | 目标文件 |
| `exe` | 可执行文件 |

## 项目结构

```
my_project/
├── src/
│   └── main.at
├── lib/              # 标准库模块
├── examples/         # 示例文件
└── atom.toml         # 项目配置（可选）
```

`.at` 为 Action 源文件扩展名。

### 项目配置（atom.toml）

```toml
[project]
name = "my_project"
version = "0.1.0"
main = "src/main.at"

[build]
optimize = true
target = "native"

[profile.release]
opt_level = 3
lto = true
```

## 编译器架构

```
源文件 (.at)
  → Lexer      词法分析，生成 Token 流
  → Parser     Pratt 解析器，生成 AST
  → TypeChecker 类型检查与推断（含智能转换）
  → Codegen    LLVM IR 生成（基于 inkwell）
  → JIT / AOT  即时执行或编译为目标代码
  → LSP        Language Server Protocol 支持
```

## 从源码构建

```bash
# 依赖: Rust 1.70+, LLVM 21+, cmake, pkg-config
git clone https://github.com/TetraploidHuman/Action.git
cd Action
cargo build --release
```

## 许可证

MIT License
