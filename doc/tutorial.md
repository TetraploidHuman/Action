# Action Language 教程与开发手册

> 本文档既可作为入门教程，也可作为日常开发参考手册。
> 所有代码示例均基于实际编译器行为验证。

---

# 第一章：概述与环境搭建

## 1.1 关于 Action

Action 是一门静态类型的多范式编程语言，编译器使用 Rust 编写，基于 LLVM 后端，支持 JIT 即时编译与原生代码生成。

**设计哲学：**
- 实用性优先，兼顾表达力与性能
- 可失败（fallible）与 `or { }` — 见第十一章
- 函数式风格的一等函数与集合操作
- 模式匹配作为核心控制流机制
- 轻量级协程支持异步通信

## 1.2 安装与编译

### 前置依赖

- Rust 工具链（1.70+）
- LLVM 21+
- cmake、pkg-config

### 从源码构建

```bash
git clone https://github.com/TetraploidHuman/Action.git
cd Action
cargo build --release
```

### 验证安装

```bash
./target/release/action run examples/hello.ac
# 输出: Hello, World!
```

## 1.3 命令行工具

| 命令 | 作用 |
|------|------|
| `action run file.ac` | 编译并运行（JIT） |
| `action build file.ac` | 编译为 LLVM IR |
| `action build file.ac -o prog` | 编译为可执行文件 |
| `action check file.ac` | 仅类型检查，不运行 |
| `action repl` | 启动交互式 REPL |
| `action lsp` | 启动 Language Server |

### 常用选项

```bash
action run file.ac --check    # 类型检查 + JIT
action run file.ac -O 3       # 优化等级 0-3
action run file.ac --emit ir  # 输出 LLVM IR
action build file.ac --emit asm   # 输出汇编
action build file.ac --emit exe   # 链接为可执行文件
```

### 交叉编译

```bash
action run file.ac --target wasm
action build file.ac --target linux-arm64
```

## 1.4 源文件与项目结构

源文件使用 `.ac` 扩展名。

```
my_project/
├── src/
│   └── main.ac
├── lib/              # 标准库模块
├── examples/         # 示例文件
└── atom.toml         # 项目配置（可选）
```

### 项目配置（atom.toml）

```toml
[project]
name = "my_project"
version = "0.1.0"
main = "src/main.ac"

[build]
optimize = true
target = "native"

[profile.release]
opt_level = 3
lto = true
```

---

# 第二章：基础语法

## 2.1 Hello World

```action
// Hello World in Action
fun main() {
    println("Hello, World!")
}
```

每个 Action 程序的入口是 `fun main()` 函数。

## 2.2 注释

```action
// 单行注释

/* 块注释
   可以跨行 */
```

## 2.3 变量绑定

### 不可变绑定（推荐）

```action
val x = 42                  // 类型自动推断
val name: String = "Action"  // 带类型标注（冒号分隔）
val typed: Int = 42
```

### 可变绑定

```action
var y = 0
y = y + 1
y += 1                      // 复合赋值
```

### 惰性绑定

```action
lazy val big = heavyComputation()  // 惰性初始化，首次访问时计算
```

### 编译期常量

```action
const MAX_SIZE: Int = 1024
const PI = 3
```

## 2.4 基本类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `Int` | 64位有符号整数 | `42`, `-1`, `0` |
| `Float` | 64位浮点数 | `3.14`, `-0.5` |
| `Bool` | 布尔值 | `true`, `false` |
| `String` | 字符串 | `"hello"` |
| `Char` | 字符 | `'A'`, `'0'` |
| `()` | 单元类型 | `()` |

```action
val i: Int = 42
val f: Float = 3.14
val b: Bool = true
val s: String = "hello"
val c: Char = 'A'
```

### 特殊数字字面量

```action
val bin = 0b1010      // 二进制 = 10
val oct = 0o777       // 八进制 = 511
val hex = 0xFF        // 十六进制 = 255
val big = 1_000_000   // 分隔线（可读性）
```

## 2.5 字符串插值

```action
val name = "World"
val age = 42

println("Hello, ${name}!")           // Hello, World!
println("Age: ${age}")               // Age: 42
println("${name} is ${age} years old") // World is 42 years old
```

只支持 `${expr}` 形式，不支持 `$name` 简写。

## 2.6 类型别名

```action
type UserId = Int
type Person = { id: UserId, name: String }
type Callback = (Int) -> Bool   // 函数类型别名
```

## 2.7 元组

元组使用圆括号 `()` 构造，支持任意数量的元素和混合类型：

```action
val t = (1, 2, 3)
print(t[0])   // 1
print(t[1])   // 2
print(t[2])   // 3

val pair = (42, "hello")  // 混合类型
print(pair[0])   // 42
```

元组解构：

```action
val pair = (42, 10)
val (x, y) = pair
print(x)  // 42
print(y)  // 10
```

`to` 运算符创建二元元组：

```action
val pair = 1 to "a"   // (1, "a")
```

元组可用于 for 表达式中：

```action
val pairs = for x in List[1, 2], y in List["a", "b"] { x to y }
```

---

# 第三章：运算符

## 3.1 算术运算符

```action
a + b    a - b    a * b    a / b    a % b
a ** b   // 幂运算
```

复合赋值：`+=` `-=` `*=` `/=` `%=`

## 3.2 比较运算符

```action
a == b    a != b    a < b    a > b    a <= b    a >= b
```

## 3.3 逻辑运算符（短路求值）

```action
a and b   // 逻辑与
a or b    // 逻辑或
not a     // 逻辑非
```

## 3.4 位运算符

```action
a & b    // 按位与
a | b    // 按位或
a ^ b    // 按位异或
a << b   // 左移
a >> b   // 右移
~a       // 按位取反
```

## 3.5 类型检查运算符

```action
x is Int       // 检查 x 是否为 Int 类型
x in list      // 检查 x 是否在集合中
```

## 3.6 范围运算符

```action
1..5           // 1, 2, 3, 4, 5（包含 5）
1..<5          // 1, 2, 3, 4（不包含 5）
```

范围可以用于 for 循环和值匹配。

---

# 第四章：控制流

## 4.1 When 表达式

`when` 是 Action 的核心控制流机制，替代了传统的 `if/else` 和 `switch`。

### 单行 when（三元条件）

```action
val max = when a > b { a else b }
val abs = when x < 0 { -x else x }
```

`else` 分支可省略（省略时默认返回 `()`）：

```action
when debug { println("debug: ${x}") }
```

### 条件链 when

```action
val grade = when {
    score >= 90 -> "A"
    score >= 80 -> "B"
    score >= 70 -> "C"
    else -> "D"
}
```

### 值匹配 when

```action
val desc = when x {
    1 -> "one"
    2 -> "two"
    3 -> "three"
    else -> "many"
}
```

支持枚举/结构体等值匹配（`null` 模式已移除，见第十一章 fallible）。

### 带守卫的模式匹配

```action
val r = when x {
    n and n > 0 -> "positive"
    n and n < 0 -> "negative"
    else        -> "zero"
}
```

### 或模式

```action
val c = when color {
    Red, Green -> "warm"
    Blue       -> "cool"
}

val desc = when value {
    0 -> "zero"
    else -> "non-zero"
}
```

### when 中的解构

```action
val area = when shape {
    Circle(r)   -> 3.14 * r * r
    Rectangle(w, h) -> w * h
}

// is 类型检查
val description = when value {
    is Int   -> "integer: ${value}"
    is Float -> "float: ${value}"
    else     -> "other"
}
```

## 4.2 For 循环

### 迭代集合

```action
for item in list {
    println(item)
}

for item in List[1, 2, 3] {
    print(item)
}
```

### 范围迭代

```action
// 含上界
for i in 1..5 {
    print(i)       // 1, 2, 3, 4, 5
}

// 不含上界
for i in 1..<5 {
    print(i)       // 1, 2, 3, 4
}
```

### 带索引迭代

```action
for index, item in list {
    println("${index}: ${item}")
}
```

### 条件循环（类似 while）

```action
var i = 0
for i < 10 {
    i = i + 1
}
```

### 无限循环

```action
for {
    when condition { break else continue }
}
```

### For 表达式（收集结果）

For 可以作为表达式，将每次迭代的结果收集到新列表中：

```action
val doubled = for x in list { x * 2 }
```

带过滤（使用 `continue`）：

```action
val evens = for x in list {
    when x % 2 == 0 { x else continue }
}
```

### 简写形式（隐式 it）

```action
val cubes = for List[1, 2, 3] { it * it * it }
```

### 嵌套 For

```action
val pairs = for x in List[1, 2], y in List["a", "b"] { x to y }
// results: [{1, "a"}, {1, "b"}, {2, "a"}, {2, "b"}]
```

---

# 第五章：函数

## 5.1 函数定义

### 单表达式函数

```action
fun add(a: Int, b: Int) -> Int = a + b
```

返回类型可省略（推断）：

```action
fun greet(name: String) = println("Hello, ${name}!")
```

### 多语句函数（块体）

```action
fun factorial(n: Int) -> Int {
    when n {
        0 -> 1
        else -> n * factorial(n - 1)
    }
}
```

### 参数类型标注

函数参数使用冒号标注类型：

```action
fun add(x: Int, y: Int) -> Int = x + y
```

## 5.2 递归与尾递归优化

```action
fun fib(n: Int) -> Int {
    when n <= 1 { n else fib(n - 1) + fib(n - 2) }
}

// 尾递归优化（自动）
fun sum(n: Int, acc: Int) -> Int {
    when n <= 0 { acc else sum(n - 1, acc + n) }
}
```

## 5.3 函数重载

同名函数可通过参数类型区分：

```action
fun add(x: Int, y: Int) -> Int = x + y
fun add(x: Float, y: Float) -> Float = x + y

fun main() {
    println(add(1, 2))      // 3
    println(add(1.5, 2.5))  // 4.0
}
```

## 5.4 函数类型与引用

### 函数类型

```action
// 函数类型签名: (参数类型) -> 返回类型
type MathOp = (Int) -> Int
```

### 高阶函数

```action
fun apply_twice(f: (Int) -> Int, x: Int) -> Int {
    f(f(x))
}

fun double(n: Int) -> Int = n * 2

fun main() {
    val r = apply_twice(double, 5)
    print(r)  // 20
}
```

## 5.5 导出函数

```action
export fun helper() -> Int = 42

// 导出的函数可以被其他模块 import
```

---

# 第六章：Lambda 与高阶函数

## 6.1 Lambda 表达式

### 无参 Lambda

```action
val answer = { 42 }
print(answer())   // 调用时使用 ()
```

### 显式参数 Lambda

```action
val add = { x, y -> x + y }
print(add(10, 20))  // 30
```

带类型标注的参数：

```action
val add = { x: Int, y: Int -> x + y }
print(add(10, 20))  // 30
```

### 隐式 it 参数

当 Lambda 只有一个参数时，可以用 `it` 代替：

```action
val double = { it * 2 }
print(double(21))   // 42
```

### 直接调用

```action
print({ x, y -> x * y }(6, 7))  // 42
```

## 6.2 集合高阶函数

Action 使用顶层函数进行集合操作，lambda 作为最后一个参数：

```action
val nums = List[1, 2, 3, 4, 5]

val doubled = map(nums) { it * 2 }
val evens = filter(nums) { it % 2 == 0 }
val sum = fold(nums, 0) { acc, x -> acc + x }
val allPositive = all(nums) { it > 0 }
val hasEven = any(nums) { it % 2 == 0 }
val firstEven = find(nums) { it % 2 == 0 }
```

集合的 `map`、`filter`、`fold` 等也支持方法调用语法：

```action
// 方法调用形式（仅 list/map/set 支持）
nums.get(0)
nums.contains(3)
nums.take(2)
```

## 6.3 闭包捕获

```action
var count = 0
val increment = { count += 1 }
increment()
increment()
println(count)  // 2
```

## 6.4 函数组合工具

```action
identity(x)         // 恒等函数: 返回 x
compose(f, g)       // 函数组合: f(g(x))
curry(f, x)         // 柯里化
uncurry(f)          // 反柯里化
flip(f)             // 交换参数顺序
constant(x)         // 返回常函数
```

---

# 第七章：集合类型

Action 提供三种内置集合类型：List、Map、Set。所有集合操作以**顶层函数**形式提供，同时也支持方法调用语法。

## 7.1 List（列表）

### 创建

```action
val nums = List[1, 2, 3, 4, 5]
val empty = List[]           // 空列表
val squares = for x in 1..5 { x * x }  // for 表达式创建
```

### 基本操作

```action
// 索引访问（越界须 or { }）
nums[0] or { -1 }              // 1
get(nums, 0) or { -1 }         // 1

// 大小与判空
len(nums)            // 5
isEmpty(nums)        // false

// 元素访问（空列表/越界须 or { }）
head(nums) or { -1 }           // 1
last(nums) or { -1 }           // 5
tail(nums) or { List[] }       // List[2, 3, 4, 5]（fallback 类型须与列表元素一致）
init(nums) or { List[] }       // List[1, 2, 3, 4]

// 查询
contains(nums, 3)    // true
indexOf(nums, 3) or { -1 }     // 2
```

### 转换操作

```action
reverse(nums)        // List[5, 4, 3, 2, 1]
sorted(nums)         // List[1, 2, 3, 4, 5]
unique(nums)         // 去重
flatten(lists)       // 展平嵌套列表
take(nums, 2)        // List[1, 2]
drop(nums, 2)        // List[3, 4, 5]
append(nums, 6)      // List[1, 2, 3, 4, 5, 6]
prepend(nums, 0)     // List[0, 1, 2, 3, 4, 5]
zip(nums, other)     // List[{1, "a"}, {2, "b"}, ...]
withIndex(nums)      // List[{0, 1}, {1, 2}, ...]
```

### 高级操作

```action
slice(nums, 1, 3)    // List[2, 3]
splitAt(nums, 2)     // [{1, 2}, {3, 4, 5}]
chunks(nums, 2)      // [{1, 2}, {3, 4}, {5}]
windows(nums, 2)     // [{1, 2}, {2, 3}, {3, 4}, {4, 5}]
repeat(nums, 3)      // List[1, 2, 3, 1, 2, 3, 1, 2, 3]
```

### 可变操作

```action
// insert 和 remove 在原列表上修改
insert(list, 0, 99)  // 在索引 0 处插入 99
remove(list, 0)     // 删除索引 0 的元素
```

### Lambda 回调函数

```action
map(list) { it * 2 }          // 映射
filter(list) { it % 2 == 0 }  // 过滤
fold(list, init) { acc, x -> ... }  // 折叠
reduce(list) { acc, x -> ... }  // 归约（无初始值）
any(list) { it > 3 }          // 任一满足
all(list) { it > 0 }          // 全部满足
find(list) { it == 3 }        // 查找第一个
findIndex(list) { it == 3 }   // 查找索引
flatMap(list) { List[it, it*2] }  // 扁平映射
sortedBy(list) { -it }        // 自定义排序
partition(list) { it % 2 == 0 }  // 分区
count(list) { it > 0 }        // 计数
takeWhile(list) { it < 4 }    // 条件取
dropWhile(list) { it < 3 }    // 条件丢
```

### 方法调用形式

List 的大部分操作也支持方法调用语法：

```action
list.len()
list.head()
list.contains(3)
list.map { it * 2 }       // 注意: map/filter/fold 需通过顶层函数
```

## 7.2 Map（映射）

### 创建

```action
val m = Map["a": 1, "b": 2]
val empty = Map[]          // 空映射
```

### 基本操作（顶层函数）

```action
get(m, "a") or { -1 }              // 1
containsKey(m, "a")      // true
insert(m, "c", 3)        // 插入/更新
remove(m, "a")           // 删除键
len(m)                   // 2
isEmpty(m)               // false
mapKeys(m)               // List["a", "b"]
mapValues(m)             // List[1, 2]
mapEntries(m)            // List[{"a", 1}, {"b", 2}]
mapUnion(m1, m2)         // 合并
mapFilter(m) { fn }      // 过滤
mapFold(m, init) { fn }  // 折叠
mapMapValues(m) { fn }   // 值映射
```

### 方法调用形式

Map 也支持方法调用语法：

```action
m.contains("a")          // true
m.len()                  // 2
m.keys()                 // List["a", "b"]
m.values()               // List[1, 2]
m.entries()              // List[{"a", 1}, {"b", 2}]
m.insert("c", 3)
m.remove("a")
m.union(other)
m.filter { fn }
m.fold(init) { fn }
m.mapValues { fn }
m.isEmpty()              // false
```

## 7.3 Set（集合）

### 创建

```action
val s = Set[1, 2, 3, 3, 2]  // Set[1, 2, 3]（自动去重）
val empty = Set[]           // 空集合
```

### 基本操作

```action
// 顶层函数方式
contains(s, 3)          // true
len(s)                  // 2
isEmpty(s)              // false
setUnion(s1, s2)        // 并集
setIntersection(s1, s2) // 交集
setDifference(s1, s2)   // 差集
setIsSubset(s1, s2)     // 子集判断

// 方法调用方式
s.contains(3)
s.len()
s.isEmpty()
s.insert(4)             // 插入
s.remove(2)             // 删除
s.union(other)
s.intersection(other)
s.difference(other)
s.is_subset(other)
s.toList()              // 转列表
```

---

# 第八章：字符串

字符串操作以**顶层函数**形式提供，不支持方法调用语法（如 `s.split(",")` 不可用，需使用 `split(s, ",")`）。

## 8.1 基本操作

```action
val s = "Hello, Action!"

len(s)                  // 14（字符串长度）
isEmpty(s)              // false
toUpper(s)              // "HELLO, ACTION!"
toLower(s)              // "hello, action!"
trim(s)                 // 去除两端空白
trimStart(s)            // 去除左端空白
trimEnd(s)              // 去除右端空白
```

## 8.2 查询

```action
startsWith(s, "Hello")  // true
endsWith(s, "Action!")  // true
contains(s, "Action")   // true
stringContains(s, "Action") // true（别名）
indexOf(s, "Action") or { -1 }     // 7
charAt(s, 1)            // Char 'e'
charCode(s, 0)          // Int = 72 ('H' 的 ASCII)
chars(s)                // List['H', 'e', 'l', ...]
```

## 8.3 转换

```action
substring(s, 7, 6)      // "Action"（从索引 7 开始取 6 个字符）
split(s, ", ")          // List["Hello", "Action!"]
splitLines(s)           // 按换行分割
join(List["a", "b"], ",") // "a,b"
replace(s, "Hello", "Hi") // "Hi, Action!"
repeat(s, 3)            // 重复 3 次拼接
concat(s, "!!")         // 字符串拼接

toInt("42") or { -1 }              // 42
toFloat("3.14") or { 0.0 }         // 3.14
toString(42)            // "42"
isAlpha('A')            // true
toChar(65) or { 0 }                // Char 'A'
codeToChar(65)          // Char 'A'
```

## 8.4 字符串插值

```action
val name = "World"
val msg = "Hello, ${name}!"  // "Hello, World!"
```

---

# 第九章：结构体

## 9.1 类型定义

```action
type Point = { x: Int, y: Int }
```

## 9.2 构造

```action
// 完整字段构造
val p = { x = 10, y = 20 }

// 简写构造（变量名与字段名一致时）
val x = 10
val y = 20
val p2 = { x, y }
```

## 9.3 字段访问

```action
val px = p.x      // 10
val py = p.y      // 20
```

## 9.4 解构

```action
val {x, y} = p
val {x as px, y as py} = p  // 重命名
```

## 9.5 嵌套结构体

```action
type Address = { city: String, street: String }
type Person = { name: String, addr: Address }

val p = {
    name = "Alice",
    addr = { city = "Beijing", street = "Main St" }
}

val city = p.addr.city  // 嵌套访问
```

---

# 第十章：枚举与模式匹配

## 10.1 枚举定义

```action
// 简单枚举
enum Color {
    Red,
    Green,
    Blue
}

// 带数据的枚举
enum Shape {
    Circle(Int),        // 半径
    Rectangle(Int, Int) // 宽, 高
}

// 泛型枚举
enum Option[T] {
    Some(T),
    None,
}
```

## 10.2 构造

```action
val c = Color.Red                // 完整构造
val r = Color.Red                // 类型可推断时省略枚举名
val s = Shape.Circle(10)
val opt = Option.Some(42)
val none = Option.None
```

## 10.3 值匹配

```action
val colorName = when c {
    Color.Red   -> "red"
    Color.Green -> "green"
    Color.Blue  -> "blue"
}
```

## 10.4 解构匹配

```action
val area = when shape {
    Shape.Circle(r)     -> 3.14 * r * r
    Shape.Rectangle(w, h) -> w * h
}
```

## 10.5 守卫条件

```action
val result = when opt {
    Option.Some(v) and v > 0 -> "positive: ${v}"
    Option.Some(_)           -> "non-positive"
    Option.None              -> "empty"
}
```

## 10.6 或模式

```action
val warm = when color {
    Color.Red, Color.Orange -> true
    else                    -> false
}
```

## 10.7 穷尽性检查

编译器会检查 `when` 表达式是否穷尽所有枚举变体。不穷尽的匹配需要 `else` 分支：

```action
// 编译错误: 需要 else 分支
val name = when c {
    Color.Red -> "red"
    Color.Green -> "green"
    // 缺少 Blue 分支
}

// 正确:
val name = when c {
    Color.Red -> "red"
    Color.Green -> "green"
    Color.Blue -> "blue"
}
```

---

# 第十一章：可失败（fallible）与 `or {}`

Action **不支持** `null`、`T?` 与 `?.`（编译期报错 **E010** / **E011** / **E012**）。可能失败的操作须用 `or { }` 提供默认值或 `or { return expr }` 传播失败。

## 11.1 表达式级 `or {}`

```action
val first = head(nums) or { -1 }
val n = parseInt("42") or { 0 }
val line = readLine() or { "EOF" }
```

## 11.2 函数级 `or { }`

```action
fun parseLine(s: String) -> Int {
    parseInt(s)
} or { -1 }
```

## 11.3 错误码

| 码 | 含义 |
|----|------|
| **E001** | 裸 fallible 调用 |
| **E002** | `or {}` 类型不匹配 |
| **E003** | 函数级 `or {}` 与返回类型不符 |
| **E010** | 使用 `null` |
| **E011** | 使用 `T?` |
| **E012** | 使用 `?.` |

---

# 第十二章：泛型

## 12.1 泛型函数

```action
fun <T> identity(x: T) -> T = x
fun <T> pickFirst(a: T, b: T) -> T = a

fun main() {
    val x = identity(42)      // Int
    val y = identity("hello") // String
    val z = pickFirst(1, 2)  // Int
}
```

## 12.2 泛型枚举

```action
enum Option[T] {
    Some(T),
    None,
}

enum Pair[A, B] {
    Pair(A, B)
}
```

## 12.3 泛型集合

```action
val list = List[1, 2, 3]       // List[Int]
val map = Map["a": 1, "b": 2] // Map[String, Int]
```

---

# 第十三章：扩展方法

## 13.1 定义扩展

```action
extension Int {
    fun double(self) -> Int = self * 2
    fun isEven(self) -> Bool = self % 2 == 0
    fun add(self, other: Int) -> Int = self + other
}
```

## 13.2 使用扩展

```action
fun main() {
    val x = 5
    println(x.double())  // 10
    println(x.isEven())  // false
    println(x.add(3))    // 8
}
```

扩展方法的第一个参数 `self` 指向接收者实例。

---

# 第十四章：模块系统

## 14.1 模块文件

Action 的模块系统基于源文件。标准库位于 `lib/` 目录：

```
lib/
├── math.ac       # 数学运算
└── json.ac       # JSON 解析
```

## 14.2 导入模块

```action
// 导入整个模块（通过模块名访问）
import math
math.add(10, 5)
math.sub(10, 3)

// 选择性导入
import math.{add, sub}
add(10, 5)

// 带别名导入
import math as m
m.add(10, 5)
```

## 14.3 导出项目

```action
export fun helper() -> Int = 42
export const VERSION = "1.0"
```

---

# 第十五章：协程与流

## 15.1 启动任务

```action
val task = launch {
    println("Hello from task!")
}
```

## 15.2 Stream（流）

Stream 是任务间通信的通道：

```action
val s = Stream()          // 创建流

// 发送端
launch {
    send(s, 42)
    send(s, 100)
    close(s)               // 关闭流
}

// 接收端
val first = receive(s)    // 42
val second = receive(s)   // 100
```

## 15.3 任务操作

```action
cancel(task)        // 取消任务
is_done(task)       // 检查是否完成
is_cancelled(task)  // 检查是否被取消
wait(task)          // 等待任务完成
```

## 15.4 协程作用域

```action
coroutineScope {
    // 结构化并发
    launch { ... }
    launch { ... }
}
```

---

# 第十六章：JSON 支持

推荐使用 `lib/json.ac` 模块（`import json`），其公开 API 与底层 `action_json_*` FFI 一一对应：

| 模块函数 | FFI |
|----------|-----|
| `jsonParse` | `action_json_parse` |
| `jsonStringify` | `action_json_stringify` |
| `jsonFree` | `action_json_free` |
| `jsonType` | `action_json_type` |
| `jsonGet` | `action_json_get` |
| `jsonGetIdx` | `action_json_get_idx` |
| `jsonAsStr` | `action_json_as_str` |
| `jsonAsFloat` | `action_json_as_float` |
| `jsonAsBool` | `action_json_as_bool` |
| `jsonLen` | `action_json_len` |

## 16.1 解析

```action
import json.{jsonParse, jsonFree, jsonType}

val root = jsonParse("{\"a\": 1, \"b\": 2}")
```

## 16.2 访问

```action
import json.{jsonGet, jsonAsFloat, jsonType}

val a = jsonGet(root, "a")
val aVal = jsonAsFloat(a)  // 1.0
val t = jsonType(root)     // 类型常量
```

## 16.3 序列化

```action
import json.jsonStringify

val str = jsonStringify(root)
jsonFree(root)
```

### 类型常量（`lib/json.ac`）

| 常量 | 值 | 说明 |
|------|-----|------|
| `JSON_NULL` | 0 | null |
| `JSON_BOOL` | 1 | 布尔值 |
| `JSON_NUMBER` | 2 | 数字 |
| `JSON_STRING` | 3 | 字符串 |
| `JSON_ARRAY` | 4 | 数组 |
| `JSON_OBJECT` | 5 | 对象 |

---

# 第十七章：HTTP 请求与网络

`httpRequest` 返回 `HttpResponse { status: Int, body: String }`，为 **fallible** 内置：连接/传输失败时 `status == 0`，必须用 `or {}` 处理：

```action
val resp = httpRequest(
    "GET",
    "https://httpbin.org/get",
    "Accept: application/json",
    ""
) or { { status = 0, body = "" } }

println(resp.status)
println(resp.body)

// 函数级回退
fun safeGet(url: String) -> HttpResponse {
    httpRequest("GET", url, "", "")
} or { { status = 0, body = "unreachable" } }
```

`stdlib/http.atom` 提供 `httpGet` / `httpPost` 包装（内部已带 `or {}` 回退）。

```action
// 网络连通性测试（FFI 示例，需链接测试运行时）
external fun action_test_ping() -> Int
action_test_ping()
```

---

# 第十八章：文件 I/O

## 18.1 基本文件操作

```action
writeFile("/tmp/test.txt", "hello")
appendFile("/tmp/test.txt", " world\n")
val content = readFile("/tmp/test.txt")   // 读取全部内容
exists("/tmp/test.txt")                    // true
deleteFile("/tmp/test.txt")               // 删除
```

## 18.2 流式文件操作

```action
val f = openFile("/tmp/test.txt", "r")    // 打开文件
val line = fileReadLine(f)                 // 读取一行
val bytes = fileReadBytes(f, 4096)         // 读取最多 4096 字节（二进制）
fileWrite(f, "data")                       // 写入
fileWriteLine(f, "line")                   // 写入一行
fileFlush(f)                               // 刷新缓冲区
fileSeek(f, 0)                             // 定位
fileTell(f)                                // 当前位置
isEof(f)                                   // 是否到结尾
closeFile(f)                               // 关闭文件
```

## 18.3 标准输入

```action
val name = readLine() or { "EOF" }
```

## 18.4 目录操作

```action
val entries = readDir("/tmp")  // 列出目录内容
```

---

# 第十九章：FFI（外部函数接口）

## 19.1 声明外部函数

```action
external fun printf(format: string, ...) -> Int
external fun action_test_ping() -> Int
```

## 19.2 声明外部类型

```action
external type FILE
external type FileHandle
```

## 19.3 C 字符串操作

```action
val cs = toCString("hello")   // String → CString
val s = fromCString(ptr)      // CString → String
val isNull = isNull(ptr)      // 检查空指针
val val = deref(ptr)          // 解引用指针
```

---

# 第二十章：数学函数

## 20.1 基础数学

```action
abs(-5)          // 5
min(3, 7)        // 3
max(3, 7)        // 7

clamp(0.5, 0.0, 1.0)  // 限制在区间内
```

## 20.2 幂与根

```action
pow(2.0, 10.0)   // 1024.0
sqrt(16.0)       // 4.0
cbrt(27.0)       // 3.0
```

## 20.3 三角函数

```action
sin(x)    cos(x)    tan(x)
asin(x)   acos(x)   atan(x)
atan2(y, x)
```

## 20.4 舍入

```action
floor(3.7)       // 3.0
ceil(3.2)        // 4.0
round(3.5)       // 4.0
```

## 20.5 对数与指数

```action
log(10.0)        // 自然对数
log2(8.0)        // 3.0
log10(100.0)     // 2.0
exp(1.0)         // e
```

## 20.6 常量与判断

```action
pi()             // π
e()              // e
isNaN(x)         // 是否为 NaN
isInfinite(x)    // 是否无穷
```

## 20.7 随机数

```action
randInt(1, 100)  // 1-100 之间的随机整数
randFloat()      // 0.0-1.0 之间的随机浮点数
randChoice(list) // 随机选择一个元素
randShuffle(list) // 随机打乱列表
```

## 20.8 数字工具

```action
digits(12345)    // List[1, 2, 3, 4, 5]
toChar(65)       // Char 'A'
codeToChar(65)   // Char 'A'
isAlpha('A')     // true
```

---

# 第二十一章：日期与时间

```action
val now = now()              // 当前时间
val today = today()          // 当前日期
val utc = nowUtc()           // UTC 时间

// 日期构造（使用结构体字面量）
val d = {year = 2026, month = 6, day = 1}
val dt = {year = 2026, month = 6, day = 1,
          hour = 12, minute = 30, second = 0}

// 日期操作
year(d)     month(d)    day(d)
hour(dt)    minute(dt)  second(dt)
weekday(dt)             // 星期几

// 日期运算
addDays(d, 7)
addHours(dt, 3)
diffDays(d1, d2)
diffSeconds(dt1, dt2)

// 解析与格式化
val parsed = parseDate("2026-06-15", "yyyy-MM-dd")
val formatted = format(dt, "yyyy-MM-dd")
```

---

# 第二十二章：类型转换与工具函数

## 22.1 数值转换

```action
toInt(3.14) or { 0 }               // Float → Int: 3
toFloat(42) or { 0.0 }             // Int → Float: 42.0
toInt("42") or { 0 }               // String → Int: 42
toFloat("3.14") or { 0.0 }         // String → Float: 3.14
```

## 22.2 通用转换

```action
toString(42)      // "42"
toString(true)    // "true"

"42".toInt() or { -1 }             // 42
"3.14".toFloat() or { 0.0 }        // 3.14
```

## 22.3 列表集合转换

```action
toList(lazyList)      // LazyList → List
toLazyList(list)      // List → LazyList
setToList(set)        // Set → List
setFromList(list)     // List → Set
fromList(list)        // List → Set（别名）
```

## 22.4 调试与断言

```action
panic("fatal error")  // 触发运行时 panic
assert(x > 0)         // 断言（失败时 panic）
```

## 22.5 惰性列表

```action
val ll = toLazyList(List[1, 2, 3])
lazyHead(ll) or { -1 }            // 1
lazyTake(2, ll)       // 取前 2 个
lazyDrop(1, ll)       // 跳过 1 个
lazyMap(ll) { it * 2 }
lazyFilter(ll) { it % 2 == 0 }
lazyTakeWhile(ll) { it < 3 }
lazyZip(ll, other)
toList(ll)            // 转回普通 List
```

---

# 第二十三章：编译器架构

## 23.1 编译流水线

```
源文件 (.ac)
  → Lexer      词法分析，生成 Token 流
  → Parser     Pratt 解析器，生成 AST
  → TypeChecker 类型检查与推断（含智能转换）
  → Codegen    LLVM IR 生成（基于 inkwell）
  → JIT / AOT  即时执行或编译为目标代码
  → LSP        Language Server Protocol 支持
```

## 23.2 词法分析（Lexer）

将源代码转换为 Token 流，处理关键字、标识符、字面量和运算符。

## 23.3 语法分析（Parser）

使用 Pratt 解析算法，支持：
- 前缀和中缀运算符解析
- Lambda 与结构体字面量的消歧
- 模式匹配语法

## 23.4 类型检查（TypeChecker）

- 结构化类型系统
- 泛型类型推断
- 智能类型转换（空判断后自动提升）
- 模式穷尽性检查

## 23.5 代码生成（Codegen）

- 基于 LLVM inkwell 绑定
- JIT 即时编译
- AOT 原生代码生成
- 尾递归优化
- 引用计数内存管理

## 23.6 LSP 支持

内置 Language Server Protocol，支持：
- 语法错误诊断
- 代码补全
- 符号跳转

---

# 附录：完整函数速查

## String 函数

| 函数 | 返回 | 说明 |
|------|------|------|
| `len(s)` | Int | 长度 |
| `isEmpty(s)` | Bool | 是否为空 |
| `toUpper(s)` | String | 转大写 |
| `toLower(s)` | String | 转小写 |
| `trim(s)` | String | 去除两端空白 |
| `trimStart(s)` | String | 去除左端空白 |
| `trimEnd(s)` | String | 去除右端空白 |
| `split(s, delim)` | List[String] | 分割 |
| `splitLines(s)` | List[String] | 按换行分割 |
| `join(list, sep)` | String | 连接列表 |
| `substring(s, from, len)` | String | 取子串 |
| `startsWith(s, prefix)` | Bool | 前缀匹配 |
| `endsWith(s, suffix)` | Bool | 后缀匹配 |
| `contains(s, substr)` | Bool | 包含检查 |
| `stringContains(s, substr)` | Bool | 包含检查 |
| `replace(s, old, new)` | String | 替换 |
| `repeat(s, n)` | String | 重复 |
| `charAt(s, idx)` | Char | 取字符 |
| `charCode(s, idx)` | Int | 取编码 |
| `chars(s)` | List[Char] | 转字符列表 |
| `indexOf(s, sub)` | Int (fallible) | 查找子串 |
| `concat(a, b)` | String | 拼接 |
| `toInt(s)` | Int (fallible) | 解析整数 |
| `toFloat(s)` | Float (fallible) | 解析浮点数 |
| `toString(v)` | String | 任意值转字符串 |
| `isAlpha(c)` | Bool | 是否字母 |
| `toChar(code)` | Char | 编码转字符 |

> 注意：字符串**不支持**方法调用语法。

## List 方法

| 方法/函数 | 返回 | 说明 |
|-----------|------|------|
| `.len()` / `len(list)` | Int | 长度 |
| `.isEmpty()` / `isEmpty(list)` | Bool | 判空 |
| `.head()` / `head(list)` | T (fallible) | 首元素 |
| `.last()` / `last(list)` | T (fallible) | 尾元素 |
| `.tail()` / `tail(list)` | List[T] | 除首元素外 |
| `.init()` / `init(list)` | List[T] | 除尾元素外 |
| `.get(idx)` / `get(list, idx)` | T (fallible) | 索引访问 |
| `.contains(e)` / `contains(list, e)` | Bool | 包含检查 |
| `.indexOf(e)` / `indexOf(list, e)` | Int (fallible) | 查找索引 |
| `.append(e)` / `append(list, e)` | List[T] | 追加 |
| `.prepend(e)` / `prepend(list, e)` | List[T] | 前置 |
| `.reverse()` / `reverse(list)` | List[T] | 反转 |
| `.take(n)` / `take(list, n)` | List[T] | 取前 n |
| `.drop(n)` / `drop(list, n)` | List[T] | 去掉前 n |
| `.sorted()` / `sorted(list)` | List[T] | 排序 |
| `.unique()` / `unique(list)` | List[T] | 去重 |
| `.sum()` / `sum(list)` | Int/Float | 求和 |
| `.product()` / `product(list)` | Int/Float | 求积 |
| `.flatten()` / `flatten(list)` | List[T] | 展平 |
| `.slice(s, e)` / `slice(list, s, e)` | List[T] | 切片 |
| `.splitAt(n)` / `splitAt(list, n)` | List[List[T]] | 拆分 |
| `.chunks(n)` / `chunks(list, n)` | List[List[T]] | 分组 |
| `.windows(n)` / `windows(list, n)` | List[List[T]] | 滑动窗口 |
| `.repeat(n)` / `repeat(list, n)` | List[T] | 重复 |
| `.withIndex()` / `withIndex(list)` | List[{Int, T}] | 带索引 |
| `.zip(other)` / `zip(list, other)` | List[{T, U}] | 压缩 |
| `.insert(idx, e)` | List[T] | 插入 |
| `.remove(idx)` | List[T] | 删除 |

### Lambda 回调函数

| 函数 | 返回 | 说明 |
|------|------|------|
| `map(list) { fn }` | List[U] | 映射 |
| `filter(list) { fn }` | List[T] | 过滤 |
| `fold(list, init) { fn }` | U | 折叠 |
| `any(list) { fn }` | Bool | 任一满足 |
| `all(list) { fn }` | Bool | 全部满足 |
| `find(list) { fn }` | T (fallible) | 查找 |
| `reduce(list) { fn }` | T (fallible) | 归约 |
| `flatMap(list) { fn }` | List[U] | 扁平映射 |
| `sortedBy(list) { fn }` | List[T] | 自定义排序 |
| `partition(list) { fn }` | {List[T], List[T]} | 分区 |
| `count(list) { fn }` | Int | 计数 |
| `takeWhile(list) { fn }` | List[T] | 条件取 |
| `dropWhile(list) { fn }` | List[T] | 条件丢 |
| `findIndex(list) { fn }` | Int (fallible) | 查找索引 |

## Map 方法

| 方法 | 返回 | 说明 |
|------|------|------|
| `.len()` | Int | 大小 |
| `.isEmpty()` | Bool | 判空 |
| `.get(key)` | V (fallible) | 取值 |
| `.contains(key)` | Bool | 键存在 |
| `.insert(key, val)` | Map | 插入/更新 |
| `.remove(key)` | Map | 删除键 |
| `.keys()` | List[K] | 所有键 |
| `.values()` | List[V] | 所有值 |
| `.entries()` | List[{K, V}] | 所有条目 |
| `.union(other)` | Map | 合并 |
| `.filter { fn }` | Map | 过滤 |
| `.fold(init) { fn }` | U | 折叠 |
| `.mapValues { fn }` | Map | 值映射 |

## Set 方法

| 方法 | 返回 | 说明 |
|------|------|------|
| `.len()` | Int | 大小 |
| `.isEmpty()` | Bool | 判空 |
| `.contains(elem)` | Bool | 包含检查 |
| `.insert(elem)` | Set | 插入 |
| `.remove(elem)` | Set | 删除 |
| `.union(other)` | Set | 并集 |
| `.intersection(other)` | Set | 交集 |
| `.difference(other)` | Set | 差集 |
| `.is_subset(other)` | Bool | 子集判断 |
| `.toList()` | List[T] | 转列表 |

---

> 本文档基于 Action 编译器实际行为编写。
> 如有疑义，请以源码和示例为准。
