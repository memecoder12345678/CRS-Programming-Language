# CRS Language

A minimal, experimental scripting language implemented in Rust.

CRS is designed as a **simple, predictable, and explicit runtime system**.
The goal of the project is to explore language design, VM architecture, and compiler implementation while keeping the execution model easy to understand.

This project includes:

* a parser
* a bytecode compiler
* a virtual machine
* a standard library
* a CLI toolchain
* a comprehensive test suite

All implemented in Rust.

---

## Philosophy

CRS follows a **strict and minimal language model**.

Many scripting languages add convenience features that introduce hidden behavior in the runtime. CRS intentionally avoids this to keep the VM small and predictable.

Core principles:

• simple runtime semantics
• explicit behavior
• minimal VM complexity
• predictable bytecode generation
• easy reasoning about execution

For example:

* `++` only works on variables
* table fields are **symbols**, not variables
* functions stored in tables must be extracted before calling
* no implicit `self` or `this` binding

This keeps the runtime implementation small and avoids complex lvalue semantics.

---

## Language Overview

>
> **Note**: The CRS language only allows two keywords at the file level: `func` and `include`.
>

CRS supports common scripting features:

### Variables

```js
let x = 10;
let name = "CRS";
```

---

### Arithmetic

```js
println(5 + 3);
println(10 - 2);
println(4 * 5);
println(10 / 3);
```

---

### Conditions

```js
if (x > 5) {
    println("x is large");
} else {
    println("x is small");
}
```

---

### Loops

```js
while (i < 10) {
    i++;
}

for (let i = 0; i < 10; i = i + 1) {
    println(i);
}
```

---

### Arrays

```js
let arr = [1,2,3];

push(arr, 4);

println(arr[0]);
```

---

### Tables (Hash Maps)

```js
let user = {
    name: "Alice",
    age: 25
};

println(user.name);
```

Tables are **data containers**, not objects.

---

### Functions

```go
func add(a, b) {
    return a + b;
}

println(add(10,20));
```

Functions are **first-class values** and can be stored in variables or tables.

---

## Function Values in Tables

CRS does not support calling a function directly from a table field.

Example (not supported):

```js
obj.fn();
```

Instead, the function must be extracted first:

```js
let f = obj.fn;
f(obj);
```

This keeps function calls explicit and avoids hidden `self` binding.

---

## Variables vs Symbols

CRS distinguishes between **variables** and **symbols**.

### Variables

A variable is a name bound in the current scope.

```js
let x = 10;
x++;
```

Variables map directly to storage slots in the VM.

---

### Symbols (table fields)

Table fields are **symbol lookups**, not variables.

```js
obj.value;
arr[0];
```

These represent **access paths inside data structures**, not scope bindings.

Because of this:

```js
obj.value++;
```

is not part of the language grammar.

Instead use:

```js
obj.value += 1;
```

which the compiler desugars into:

```js
obj.value = obj.value + 1;
```

---

### Error Handling

CRS supports `try` / `catch`.

```js
try {
    throw "Error";
} catch (err) {
    println(err);
}
```

---

### Modules

CRS supports a simple module system using `include`.

```js
include "math.crs";
include "bank.crs";
```

The compiler handles:

* loading source files
* dependency resolution
* circular dependency checks

---

## CRS Standard Library

CRS includes a built-in standard library implemented in native Rust functions.
These functions provide utilities for **I/O, strings, arrays, tables, system access, randomness, and type conversion**.

---

### GC

#### `gc_collect()`

Manually triggers the garbage collector.

```js
gc_collect();
```

Returns:

```
null
```

---

### Process

#### `quit(code)`

Terminates the program with the given exit code.

```js
quit(0);
```

Parameters:

| Name | Type | Description |
| ---- | ---- | ----------- |
| code | Int  | exit status |

---

### Console I/O

#### `print(...)`

Prints values without newline.

```js
print("Hello");
print("A", "B", 123);
```

---

#### `println(...)`

Prints values with a newline.

```js
println("Hello World");
println("A", "B", 123);
```

---

#### `input(prompt?)`

Reads a line from standard input.

```js
name = input("Enter name: ");
```

Returns:

```
String
```

---

### File I/O

#### `read(filename)`

Reads a file into a string.

```js
content = read("file.txt");
```

Returns:

```
String
```

Error if file cannot be read.

---

#### `write(filename, content)`

Writes content to a file.

```js
write("hello.txt", "Hello World");
```

---

#### `is_file_exists(filename)`

Checks if a file exists.

```js
is_file_exists("data.txt");
```

Returns:

```
Bool
```

---

### System

#### `get_env(name)`

Gets an environment variable.

```js
path = get_env("PATH");
```

Returns:

```
String | null
```

---

#### `set_env(name, value)`

Sets an environment variable.

```js
set_env("DEBUG", "1");
```

---

#### `get_dir()`

Returns current working directory.

```js
dir = get_dir();
```

Returns:

```
String
```

---

#### `change_dir(path)`

Changes working directory.

```js
change_dir("C:/projects");
```

---

#### `sys(command)`

Executes a system command and returns stdout.

```js
result = sys("ls");
```

On Windows it uses:

```
cmd /C
```

On Unix:

```
sh -c
```

Returns:

```
String
```

---

#### `is_windows_os()`

Returns true if running on Windows.

```js
is_windows_os();
```

Returns:

```
Bool
```

---

### Strings

#### `replace(string, from, to)`

Replaces occurrences of a substring.

```js
replace("hello world", "world", "CRS");
```

Result:

```
"hello CRS"
```

---

#### `split(string, delimiter)`

Splits a string into an array.

```js
split("a,b,c", ",");
```

Result:

```
["a","b","c"]
```

---

#### `slice(value, start, end?)`

Slices strings or arrays.

```js
slice("hello", 1, 4);
```

Result:

```
"ell"
```

Also works with arrays.

---

#### `strip(string)`

Removes leading and trailing whitespace.

```js
strip("   hello  ");
```

Result:

```
"hello"
```

---

### Arrays

#### `push(array, value)`

Appends value to array.

```js
push(arr, 10);
```

---

#### `pop(array)`

Removes and returns the last element.

```js
value = pop(arr);
```

---

#### `extend(array1, array2)`

Appends all elements from array2 to array1.

```js
extend(a, b);
```

---

#### `insert(array, index, value)`

Inserts value at index.

```js
insert(arr, 2, 99);
```

---

#### `len(value)`

Returns length of:

* Array
* Table
* String

```js
len(arr);
len("hello");
```

---

### Tables (Dictionary)

#### `keys(table)`

Returns all keys.

```js
keys(t);
```

---

#### `values(table)`

Returns all values.

```js
values(t);
```

---

#### `get(container, key, default?)`

Gets a value from array or table.

```js
get(arr, 1);
get(table, :name);
get(arr, 10, "default");
```

---

#### `set(container, key, value)`

Sets a value.

```js
set(arr, 1, 100);
set(table, :name, "CRS");
```

---

### Random

#### `rand_seed(seed)`

Seeds the random generator.

```js
rand_seed(123);
```

---

#### `rand()`

Returns random float in range:

```
0.0 → 1.0
```

```js
rand();
```

---

#### `rand_int(a, b)`

Returns random integer between `a` and `b`.

```js
rand_int(1, 10);
```

---

#### `rand_choice(array)`

Returns random element from array.

```js
rand_choice(arr);
```

---

### Type Conversion

#### `to_int(value)`

Converts to integer.

```js
to_int("123");
to_int(12.5);
```

---

#### `to_float(value)`

Converts to float.

```js
to_float("3.14");
```

---

#### `to_bool(value)`

Converts to boolean.

Rules:

| Value   | Result |
| ------- | ------ |
| 0       | false  |
| 0.0     | false  |
| ""      | false  |
| "false" | false  |
| null    | false  |
| others  | true   |

---

#### `to_string(value)`

Converts value to string.

```js
to_string(123);
```

---

#### `type_of(value)`

Returns value type.

```js
type_of(123);
```

Example result:

```
"Int"
"String"
"Array"
```

---

### Time

#### `get_now()`

Returns current time since Unix epoch.

```js
get_now();
```

Returns:

```
Float (seconds);
```

---


## CDA (CRS Debug Assembly)

CRS includes a custom, human-readable bytecode assembly format known as **CDA**. 

The CRS virtual machine is **register-based**. Instead of using a traditional operand stack, it operates on an unbounded set of virtual registers (`r0`, `r1`, `r2`, ...) allocated per function. This maps closely to the language's explicit variable bindings and keeps the execution model predictable.

You can inspect the generated CDA bytecode for any CRS script using the disassembly command:

```bash
crs dis <file.crs>
```

---

### Instruction Set

The CRS compiler translates scripts into a linear sequence of CDA instructions. The VM includes a complete set of operations for data movement, arithmetic, control flow, and optimizations like immediate value folding.

---

#### Loading & Copying

| Instruction | Description |
| :--- | :--- |
| `mov rD, null` | Loads a `null` value into `rD`. |
| `mov rD, true/false` | Loads a boolean value into `rD`. |
| `mov rD, #value` | Loads an immediate integer or float into `rD`. |
| `mov rD, "string"` | Loads an interned string from the StringPool into `rD`. |
| `mov rD, :symbol` | Loads an interned symbol into `rD`. |
| `mov rD, func#id` | Loads a first-class function reference into `rD`. |
| `mov rD, rS` | Copies the value from register `rS` to `rD`. |

---

#### Data Structures

| Instruction | Description |
| :--- | :--- |
| `alloc_arr rD` | Allocates a new empty array into `rD`. |
| `alloc_tab rD` | Allocates a new empty table into `rD`. |
| `mov rD,[rO + rK]` | Property/Index access. Reads `rO[rK]` into `rD`. |
| `mov [rO + rK], rV` | Property/Index assignment. Sets `rO[rK]` to `rV`. |

---

#### Arithmetic Operations

| Instruction | Description |
| :--- | :--- |
| `add rD, rA, rB` | Addition (`rD = rA + rB`). |
| `sub rD, rA, rB` | Subtraction (`rD = rA - rB`). |
| `mul rD, rA, rB` | Multiplication (`rD = rA * rB`). |
| `div rD, rA, rB` | Division (`rD = rA / rB`). |

---


#### Immediate Arithmetic (Optimized)
*Used when the right operand is a constant integer.*

| Instruction | Description |
| :--- | :--- |
| `add_imm rD, rS, #imm` | Immediate addition (`rD = rS + imm`). |
| `sub_imm rD, rS, #imm` | Immediate subtraction (`rD = rS - imm`). |
| `mul_imm rD, rS, #imm` | Immediate multiplication (`rD = rS * imm`). |
| `div_imm rD, rS, #imm` | Immediate division (`rD = rS / imm`). |

---


#### Comparison Operations

| Instruction | Description |
| :--- | :--- |
| `cmp_lt rD, rA, rB` | Less than (`rD = rA < rB`). |
| `cmp_le rD, rA, rB` | Less than or equal (`rD = rA <= rB`). |
| `cmp_gt rD, rA, rB` | Greater than (`rD = rA > rB`). |
| `cmp_ge rD, rA, rB` | Greater than or equal (`rD = rA >= rB`). |
| `cmp_eq rD, rA, rB` | Equality (`rD = rA == rB`). |
| `cmp_ne rD, rA, rB` | Inequality (`rD = rA != rB`). |

---


#### Immediate Comparisons (Optimized)
*Used when comparing a variable against a constant integer.*

| Instruction | Description |
| :--- | :--- |
| `cmp_lt_imm rD, rS, #imm` | Immediate less than (`rD = rS < imm`). |
| `cmp_le_imm rD, rS, #imm` | Immediate less than or equal (`rD = rS <= imm`). |
| `cmp_gt_imm rD, rS, #imm` | Immediate greater than (`rD = rS > imm`). |
| `cmp_ge_imm rD, rS, #imm` | Immediate greater than or equal (`rD = rS >= imm`). |
| `cmp_eq_imm rD, rS, #imm` | Immediate equality (`rD = rS == imm`). |
| `cmp_ne_imm rD, rS, #imm` | Immediate inequality (`rD = rS != imm`). |

---


#### Unary Operations

| Instruction | Description |
| :--- | :--- |
| `not rD, rS` | Logical NOT (`rD = !rS`). |
| `neg rD, rS` | Mathematical negation (`rD = -rS`). |

---


#### Control Flow

| Instruction | Description |
| :--- | :--- |
| `jmp 0xTARGET` | Unconditional jump to `TARGET` address. |
| `jz rS, 0xTARGET` | Jump to `TARGET` if `rS` is falsy (Jump if Zero). |
| `jnz rS, 0xTARGET` | Jump to `TARGET` if `rS` is truthy (Jump if Not Zero). |

---


#### Functions & Calls

| Instruction | Description |
| :--- | :--- |
| `call func, rB, #count` | Calls a known static function `func` with `count` arguments starting from base register `rB`. |
| `callv rF, rB, #count` | Calls a dynamic function stored in register `rF`. |
| `ncall native, rB, #c` | Calls a native standard library function. |
| `fncall sys, rD` | Fast-path native call for 1-argument system functions. |
| `ret rS` | Returns the value in `rS` to the caller. |

---


#### Exceptions & Memory

| Instruction | Description |
| :--- | :--- |
| `throw rS` | Throws the value in `rS` as an exception. |
| `gc rD` | Forces garbage collection (mapped to `gc_collect()`). |

---

### Example Disassembly

Consider a simple CRS function:

```go
func add(a, b) {
    return a + b;
}
```

Running `crs dis` will output how the compiler maps parameters to registers (`r0`, `r1`) and evaluates the expression:

```text
add:
  ; registers allocated: 4
  0x0000    add        r2, r0, r1
  0x0001    ret        r2
```

---

## CLI

The CRS toolchain provides several commands:


### Run a script

```bash
crs run <file.crs> [entry]
```

---

### Disassemble bytecode

```bash
crs dis <file.crs>
```

---

### Static check

```bash
crs check <file.crs>
```

---

## Example

```go
func fibonacci(n) {
    if (n <= 1) {
        return n;
    }
    return fibonacci(n-1) + fibonacci(n-2);
}

println(fibonacci(10));
```

---

## Test Suite

The project contains a **comprehensive test suite** that validates language features, including:

* variables
* arithmetic
* logic
* control flow
* arrays
* tables
* functions
* recursion
* error handling
* compound assignments
* module system
* garbage collection
* performance

Running the test suite executes **25 feature tests** covering the full runtime behavior.

---

## Purpose

This project exists primarily for:

* learning VM design
* experimenting with language semantics
* exploring bytecode compilation
* building a small but complete scripting language

CRS intentionally prioritizes **clarity of implementation over language convenience**.

---

## License

[MIT License](LICENSE)
