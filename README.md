# Action Language

[![CI](https://github.com/TetraploidHuman/Action/actions/workflows/ci.yml/badge.svg)](https://github.com/TetraploidHuman/Action/actions/workflows/ci.yml)

Action 是一门静态类型的多范式编程语言，编译器用 Rust 编写，基于 LLVM 后端，支持 JIT 即时编译与原生代码生成。

## 特性

- **静态类型系统** — 结构化类型，类型推断，类型别名
- **模式匹配** — 穷尽性 `when` 表达式，支持枚举/结构体解构
- **一等函数** — Lambda 表达式，隐式 `it` 参数，高阶函数
- **集合类型** — List、Set、Map 及丰富的内置函数
- **字符串处理** — 支持 split/join/replace/substring 等常用操作
- **文件 I/O** — 内建文件读写、追加、删除、存在检查
- **HTTP 客户端** — 内建 HTTP 请求支持
- **协程与流** — 轻量级 Task/Stream，支持异步通信
- **FFI** — `external` 关键字，支持调用 C 函数
- **跨平台** — LLVM 后端支持 Linux x64 和 Windows x64

## 快速开始

### 下载预编译版本

从 [GitHub Releases](https://github.com/TetraploidHuman/Action/releases) 下载：

| 平台 | 包名 |
|------|------|
| Linux x64 | `action-*-linux-x64.tar.gz` |
| Windows x64 | `action-*-windows-x64.zip` |

```bash
tar xzf action-0.1.0-linux-x64.tar.gz
export PATH="$PWD/action-0.1.0-linux-x64/bin:$PATH"
action run hello.at
```

### 从源码构建

需要 Rust 工具链和 LLVM 21+：

```bash
git clone https://github.com/TetraploidHuman/Action.git
cd Action
cargo build --release
```

## 语言指南

### Hello World

```action
fun main() {
    println("Hello, World!")
}
```

```bash
action run hello.at
```

### 变量

```action
val x = 42              // 不可变绑定
var y = 0               // 可变绑定
val name: String = "Action"  // 带类型标注

y += 1                  // 复合赋值: += -= *= /=
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

// 比较（返回 Bool）
a == b   a != b   a < b    a > b    a <= b   a >= b

// 逻辑（短路求值）
a and b    a or b    !a

// 位运算
a & b    a | b    a ^ b    a << b    a >> b
```

### 字符串

```action
val s = "hello"

// 常用操作
val t = trim("  hi  ")              // "hi"
val r = replace("foo bar", "bar", "baz")  // "foo baz"
val u = toUpper("hello")           // "HELLO"
val l = toLower("HELLO")           // "hello"
val sub = substring("hello", 0, 2)  // "he"
val n = len("hello")                // 5
val parts = split("a,b,c", ",")     // List["a", "b", "c"]
val joined = join(parts, "-")       // "a-b-c"
val b = startsWith("hello", "he")  // true
val c = charAt("hello", 1)         // Char 'e'
val chars_list = chars("hi")        // List['h', 'i']
val code = charCode('A')           // 65
val ch = codeToChar(65)           // 'A'
```

### 字符串插值

```action
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

// when 作为语句
when x > 10 {
    println("large")
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
```

### Lambda 表达式

```action
val double = { x -> x * 2 }       // 显式参数
val triple = { it * 3 }           // 隐式 it 参数

// 高阶函数
val nums = List[1, 2, 3, 4, 5]
val doubled = map(nums) { it * 2 }
val evens = filter(nums) { it % 2 == 0 }
val sum = fold(0, nums) { acc, x -> acc + x }
val all_positive = all(nums) { it > 0 }
val has_even = any(nums) { it % 2 == 0 }
```

### 枚举

```action
enum Option {
    Some(Int),
    None
}

enum Result {
    Ok(String),
    Err(Int)
}

// 构造
val r1 = Ok("success")
val r2 = Err(404)

// 模式匹配解构
val v = when r1 {
    Ok(msg) -> msg,
    Err(code) -> "Error: ${code}"
}
```

### 结构体

```action
type Point = {x: Int, y: Int}

// 字面量构造
val p = {x = 10, y = 20}

// 字段访问
val px = p.x

// 解构
val {x, y} = p

// 结构体更新（创建新值）
val p2 = {x = p.x, y = p.y + 1}
```

### 类型别名

```action
type UserId = Int
type Name = String
type Person = {id: UserId, name: Name}
```

### For 循环

```action
// 遍历集合
for item in List[1, 2, 3] {
    println(item)
}

// 遍历范围
for i in 1..5 {
    print(i)
}

// 条件循环
var i = 0
for i < 10 {
    i = i + 1
}
```

### 集合

```action
// List
val list = List[1, 2, 3]
val l = len(list)                  // 长度
val first = list[0]                // 索引访问
val has = contains(list, 2)        // 包含检查
val idx = indexOf(list, 3)        // 查找索引
val more = append(list, 4)         // 追加元素
val merged = concat(list, List[5, 6])  // 合并
val taken = take(list, 2)          // 取前 n 个
val rest = drop(list, 1)           // 去掉前 n 个
val sliced = slice(list, 1, 3)     // 切片 [from, to)
val rev = reverse(list)            // 反转
val sorted = sort(list)            // 排序
val uniq = unique(list)            // 去重
val zipped = zip(list, List["a", "b", "c"])  // 压缩
val chunks = chunks(list, 2)       // 分组
val flat = flatten(List[List[1, 2], List[3, 4]])  // 展平

// Set
val s1 = Set[1, 2, 3]
val s2 = Set[2, 3, 4]
val union = setUnion(s1, s2)            // Set[1, 2, 3, 4]
val inter = setIntersection(s1, s2)     // Set[2, 3]
val diff = setDifference(s1, s2)        // Set[1]
val in_set = contains(s1, 2)             // true

// Map
val m1 = Map["a": 1, "b": 2]
val m2 = Map["b": 20, "c": 3]
val keys = mapKeys(m1)            // List["a", "b"]
val vals = mapValues(m1)          // List[1, 2]
val merged = mapUnion(m1, m2)     // Map["a": 1, "b": 20, "c": 3]
```

### 文件 I/O

```action
val f = openFile("/tmp/test.txt", "w")
writeFile(f, "hello\n")
closeFile(f)

val f2 = openFile("/tmp/test.txt", "r")
val line = readLine(f2)
closeFile(f2)

// 便捷函数
appendFile("/tmp/log.txt", "log entry\n")
val exists_bool = exists("/tmp/test.txt")     // true/false
deleteFile("/tmp/test.txt")
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
```

### 类型转换

```action
val f = toFloat(42)              // Int → Float: 42.0
val i = toInt(3.14)              // Float → Int: 3
val s = intToString(42)         // Int → String: "42"
val n = parseInt("42")           // String → Int: 42
```

### 数学函数

```action
abs(-5)          // 5
min(3, 7)        // 3
max(3, 7)        // 7
clamp(x, 0, 100) // 限制在 [0, 100]
sqrt(16.0)       // 4.0
pow(2.0, 10.0)   // 1024.0
sin(x)           // 正弦
cos(x)           // 余弦
floor(3.7)       // 3.0
ceil(3.2)        // 4.0
round(3.5)       // 4.0
log(10.0)        // 自然对数
exp(1.0)         // e
log2(8.0)        // 3.0
log10(100.0)     // 2.0
gcd(48, 18)      // 6
```

### 短路的逻辑运算

```action
// and / or 是短路的
val ok = x > 0 and y / x > 2     // 若 x > 0 为假，不计算 y / x
val safe = a or risky()          // 若 a 为真，不调用 risky()
```

### 安全访问

```action
val result = maybe?.field          // 若 maybe 有效则取字段，否则短路
val value = obj?.method(arg)       // 安全方法调用
```

### 错误传播

```action
val x? = parseInt("123")          // ? 传播错误
val y = result?                    // 后缀 try 运算符
```

### 协程与流

```action
// 创建流
val (rx, tx) = stream()

// 启动异步任务
val task = launch {
    send(tx, 42)
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

// 导出
export fun helper() { 42 }
```

### FFI

```action
// 声明外部 C 函数
external fun printf(format: String, ...): Int

// 声明外部类型
external type FileHandle
```

## 内置函数速查

### 通用
| 函数 | 签名 | 说明 |
|------|------|------|
| `len` | `(T) → Int` | List/String/Map/Set 长度 |
| `isEmpty` | `(T) → Bool` | 是否为空 |
| `print` | `(T) → void` | 打印（无换行）|
| `println` | `(T) → void` | 打印（带换行）|

### 字符串
| 函数 | 签名 | 说明 |
|------|------|------|
| `trim` | `(String) → String` | 去除两端空白 |
| `replace` | `(String, String, String) → String` | 替换子串 |
| `toUpper` | `(String) → String` | 转大写 |
| `toLower` | `(String) → String` | 转小写 |
| `split` | `(String, String) → List[String]` | 分割字符串 |
| `join` | `(List[String], String) → String` | 连接字符串 |
| `substring` | `(String, Int, Int) → String` | 取子串 (from, to) |
| `startsWith` | `(String, String) → Bool` | 前缀匹配 |
| `endsWith` | `(String, String) → Bool` | 后缀匹配 |
| `contains` | `(String/String, String/元素) → Bool` | 包含检查 |
| `charAt` | `(String, Int) → Char` | 取字符 |
| `chars` | `(String) → List[Char]` | 转为字符列表 |
| `charCode` | `(Char) → Int` | 字符 → ASCII 码 |
| `codeToChar` | `(Int) → Char` | ASCII 码 → 字符 |

### 列表
| 函数 | 签名 | 说明 |
|------|------|------|
| `List[...]` | `(...) → List[T]` | 构造列表 |
| `append` | `(List[T], T) → List[T]` | 追加元素 |
| `concat` | `(List[T], List[T]) → List[T]` | 合并列表 |
| `contains` | `(List[T], T) → Bool` | 包含检查 |
| `indexOf` | `(List[T], T) → Int` | 查找索引 |
| `reverse` | `(List[T]) → List[T]` | 反转 |
| `sort` | `(List[T]) → List[T]` | 排序 |
| `unique` | `(List[T]) → List[T]` | 去重 |
| `take` | `(List[T], Int) → List[T]` | 取前 n 个 |
| `drop` | `(List[T], Int) → List[T]` | 去掉前 n 个 |
| `slice` | `(List[T], Int, Int) → List[T]` | 切片 [from, to) |
| `zip` | `(List[A], List[B]) → List[{A, B}]` | 压缩 |
| `chunks` | `(List[T], Int) → List[List[T]]` | 分组 |
| `flatten` | `(List[List[T]]) → List[T]` | 展平 |

### 高阶函数
| 函数 | 签名 | 说明 |
|------|------|------|
| `map` | `(List[T], T → U) → List[U]` | 映射 |
| `filter` | `(List[T], T → Bool) → List[T]` | 过滤 |
| `fold` | `(U, List[T], (U, T) → U) → U` | 折叠/归约 |
| `all` | `(List[T], T → Bool) → Bool` | 全部满足 |
| `any` | `(List[T], T → Bool) → Bool` | 任一满足 |

### Set/Map
| 函数 | 说明 |
|------|------|
| `Set[...]` | 构造 Set |
| `Map[key: val, ...]` | 构造 Map |
| `setUnion(a, b)` | 并集 |
| `setIntersection(a, b)` | 交集 |
| `setDifference(a, b)` | 差集 |
| `mapUnion(a, b)` | 合并 Map（后者覆盖）|
| `mapKeys(m)` | 获取所有键 |
| `mapValues(m)` | 获取所有值 |

### 文件
| 函数 | 说明 |
|------|------|
| `openFile(path, mode)` | 打开文件 ("r"/"w") |
| `closeFile(f)` | 关闭文件 |
| `readLine(f)` | 读取一行 |
| `writeFile(f, s)` | 写入字符串 |
| `appendFile(path, s)` | 追加到文件 |
| `exists(path)` | 文件是否存在 |
| `deleteFile(path)` | 删除文件 |

### 类型转换
| 函数 | 说明 |
|------|------|
| `toFloat(Int)` | Int → Float |
| `toInt(Float)` | Float → Int |
| `intToString(Int)` | Int → String |
| `floatToString(Float)` | Float → String |
| `parseInt(String)` | String → Int |

### 数学
| 函数 | 说明 |
|------|------|
| `abs` `min` `max` `clamp` | 基本数学 |
| `sqrt` `pow` `cbrt` | 幂与根 |
| `sin` `cos` `tan` `asin` `acos` `atan` | 三角函数 |
| `floor` `ceil` `round` `trunc` | 取整 |
| `log` `log2` `log10` `exp` | 对数/指数 |
| `gcd` `lcm` | 数论 |

## 命令行

```bash
action run file.at              # 编译并运行（JIT）
action build file.at -o prog    # 编译为可执行文件
action run file.at --check      # 仅类型检查，不运行
action run file.at -O 3         # 优化等级 0-3
action build file.at --emit ir     # 输出 LLVM IR
action build file.at --emit asm    # 输出汇编
action init my_project          # 创建新项目
```

## 项目结构

```
my_project/
├── src/
│   └── main.at
└── examples/
    └── hello.at
```

`.at` 为 Action 源文件扩展名。

## 编译器架构

```
源文件 (.at)
  → Lexer      词法分析，生成 Token 流
  → Parser     Pratt 解析器，生成 AST
  → TypeChecker 类型检查与推断
  → Codegen    LLVM IR 生成（基于 inkwell）
  → JIT / AOT  即时执行或编译为目标代码
```

## 从源码构建

```bash
# 依赖: Rust 1.70+, LLVM 21+
git clone https://github.com/TetraploidHuman/Action.git
cd Action
cargo build --release
```

## 许可证

MIT License
