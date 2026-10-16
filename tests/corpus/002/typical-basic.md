# YAMDV — Reading Guide

YAMDV is a small, fast Markdown viewer for Windows. It opens a file, lays out only what you can see, and keeps memory low even for very large documents. This document is a README-like sample used for measurements (M1b, T1b) and golden images.

## 1. Section about topic 1

The reader is built around a two-level document model: a top-level block index built in one pass, and detail trees that are parsed lazily when a block becomes visible. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/1) or the [reference link][ref1] for details.

### 1.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

3. Ordered list starting at 3
4. Next number
   1. Nested ordered
   2. Another

#### 1.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 1.1.1.1 Code

```rust
fn section_1() -> u32 {
    let mut total = 0;
    for i in 0..10 {
        total += i * 1;
    }
    total
}
```

###### 1.1.1.1.1 Smallest heading

Text is shaped with DirectWrite and painted band by band into a reused bitmap, so scrolling only paints the strip that was exposed.

---

[ref1]: https://example.com/ref/1

## 2. Section about topic 2

Text is shaped with DirectWrite and painted band by band into a reused bitmap, so scrolling only paints the strip that was exposed. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/2) or the [reference link][ref2] for details.

### 2.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

4. Ordered list starting at 4
5. Next number
   1. Nested ordered
   2. Another

#### 2.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 2.1.1.1 Code

```rust
fn section_2() -> u32 {
    let mut total = 0;
    for i in 0..10 {
        total += i * 2;
    }
    total
}
```

###### 2.1.1.1.1 Smallest heading

Selections are anchored to source offsets, which means they survive relayout, resizing and scrolling without drifting.

---

[ref2]: https://example.com/ref/2

## 3. Section about topic 3

Selections are anchored to source offsets, which means they survive relayout, resizing and scrolling without drifting. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/3) or the [reference link][ref3] for details.

### 3.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

5. Ordered list starting at 5
6. Next number
   1. Nested ordered
   2. Another

#### 3.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 3.1.1.1 Code

```rust
fn section_3() -> u32 {
    let mut total = 0;
    for i in 0..10 {
        total += i * 3;
    }
    total
}
```

###### 3.1.1.1.1 Smallest heading

Every number in this project is measured on a clean runner and reported in MiB together with the raw byte count.

---

[ref3]: https://example.com/ref/3

## 4. Section about topic 4

Every number in this project is measured on a clean runner and reported in MiB together with the raw byte count. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/4) or the [reference link][ref4] for details.

### 4.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

6. Ordered list starting at 6
7. Next number
   1. Nested ordered
   2. Another

#### 4.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 4.1.1.1 Code

###### 4.1.1.1.1 Smallest heading

YAMDV is a small, fast Markdown viewer for Windows. It opens a file, lays out only what you can see, and keeps memory low even for very large documents.

---

[ref4]: https://example.com/ref/4

## 5. Section about topic 5

YAMDV is a small, fast Markdown viewer for Windows. It opens a file, lays out only what you can see, and keeps memory low even for very large documents. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/5) or the [reference link][ref5] for details.

### 5.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

7. Ordered list starting at 7
8. Next number
   1. Nested ordered
   2. Another

#### 5.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 5.1.1.1 Code

###### 5.1.1.1.1 Smallest heading

The reader is built around a two-level document model: a top-level block index built in one pass, and detail trees that are parsed lazily when a block becomes visible.

---

[ref5]: https://example.com/ref/5

## 6. Section about topic 6

The reader is built around a two-level document model: a top-level block index built in one pass, and detail trees that are parsed lazily when a block becomes visible. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/6) or the [reference link][ref6] for details.

### 6.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

8. Ordered list starting at 8
9. Next number
   1. Nested ordered
   2. Another

#### 6.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 6.1.1.1 Code

###### 6.1.1.1.1 Smallest heading

Text is shaped with DirectWrite and painted band by band into a reused bitmap, so scrolling only paints the strip that was exposed.

---

[ref6]: https://example.com/ref/6

## 7. Section about topic 7

Text is shaped with DirectWrite and painted band by band into a reused bitmap, so scrolling only paints the strip that was exposed. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/7) or the [reference link][ref7] for details.

### 7.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

9. Ordered list starting at 9
10. Next number
   1. Nested ordered
   2. Another

#### 7.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 7.1.1.1 Code

###### 7.1.1.1.1 Smallest heading

Selections are anchored to source offsets, which means they survive relayout, resizing and scrolling without drifting.

---

[ref7]: https://example.com/ref/7

## 8. Section about topic 8

Selections are anchored to source offsets, which means they survive relayout, resizing and scrolling without drifting. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/8) or the [reference link][ref8] for details.

### 8.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

10. Ordered list starting at 10
11. Next number
   1. Nested ordered
   2. Another

#### 8.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 8.1.1.1 Code

###### 8.1.1.1.1 Smallest heading

Every number in this project is measured on a clean runner and reported in MiB together with the raw byte count.

---

[ref8]: https://example.com/ref/8

## 9. Section about topic 9

Every number in this project is measured on a clean runner and reported in MiB together with the raw byte count. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/9) or the [reference link][ref9] for details.

### 9.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

11. Ordered list starting at 11
12. Next number
   1. Nested ordered
   2. Another

#### 9.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 9.1.1.1 Code

###### 9.1.1.1.1 Smallest heading

YAMDV is a small, fast Markdown viewer for Windows. It opens a file, lays out only what you can see, and keeps memory low even for very large documents.

---

[ref9]: https://example.com/ref/9

## 10. Section about topic 10

YAMDV is a small, fast Markdown viewer for Windows. It opens a file, lays out only what you can see, and keeps memory low even for very large documents. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/10) or the [reference link][ref10] for details.

### 10.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

12. Ordered list starting at 12
13. Next number
   1. Nested ordered
   2. Another

#### 10.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 10.1.1.1 Code

###### 10.1.1.1.1 Smallest heading

The reader is built around a two-level document model: a top-level block index built in one pass, and detail trees that are parsed lazily when a block becomes visible.

---

[ref10]: https://example.com/ref/10

## 11. Section about topic 11

The reader is built around a two-level document model: a top-level block index built in one pass, and detail trees that are parsed lazily when a block becomes visible. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/11) or the [reference link][ref11] for details.

### 11.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

13. Ordered list starting at 13
14. Next number
   1. Nested ordered
   2. Another

#### 11.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 11.1.1.1 Code

###### 11.1.1.1.1 Smallest heading

Text is shaped with DirectWrite and painted band by band into a reused bitmap, so scrolling only paints the strip that was exposed.

---

[ref11]: https://example.com/ref/11

## 12. Section about topic 12

Text is shaped with DirectWrite and painted band by band into a reused bitmap, so scrolling only paints the strip that was exposed. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/12) or the [reference link][ref12] for details.

### 12.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

14. Ordered list starting at 14
15. Next number
   1. Nested ordered
   2. Another

#### 12.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 12.1.1.1 Code

###### 12.1.1.1.1 Smallest heading

Selections are anchored to source offsets, which means they survive relayout, resizing and scrolling without drifting.

---

[ref12]: https://example.com/ref/12

## 13. Section about topic 13

Selections are anchored to source offsets, which means they survive relayout, resizing and scrolling without drifting. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/13) or the [reference link][ref13] for details.

### 13.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

15. Ordered list starting at 15
16. Next number
   1. Nested ordered
   2. Another

#### 13.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 13.1.1.1 Code

###### 13.1.1.1.1 Smallest heading

Every number in this project is measured on a clean runner and reported in MiB together with the raw byte count.

---

[ref13]: https://example.com/ref/13

## 14. Section about topic 14

Every number in this project is measured on a clean runner and reported in MiB together with the raw byte count. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/14) or the [reference link][ref14] for details.

### 14.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

16. Ordered list starting at 16
17. Next number
   1. Nested ordered
   2. Another

#### 14.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 14.1.1.1 Code

###### 14.1.1.1.1 Smallest heading

YAMDV is a small, fast Markdown viewer for Windows. It opens a file, lays out only what you can see, and keeps memory low even for very large documents.

---

[ref14]: https://example.com/ref/14

## 15. Section about topic 15

YAMDV is a small, fast Markdown viewer for Windows. It opens a file, lays out only what you can see, and keeps memory low even for very large documents. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/15) or the [reference link][ref15] for details.

### 15.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

17. Ordered list starting at 17
18. Next number
   1. Nested ordered
   2. Another

#### 15.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 15.1.1.1 Code

###### 15.1.1.1.1 Smallest heading

The reader is built around a two-level document model: a top-level block index built in one pass, and detail trees that are parsed lazily when a block becomes visible.

---

[ref15]: https://example.com/ref/15

## 16. Section about topic 16

The reader is built around a two-level document model: a top-level block index built in one pass, and detail trees that are parsed lazily when a block becomes visible. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/16) or the [reference link][ref16] for details.

### 16.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

18. Ordered list starting at 18
19. Next number
   1. Nested ordered
   2. Another

#### 16.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 16.1.1.1 Code

###### 16.1.1.1.1 Smallest heading

Text is shaped with DirectWrite and painted band by band into a reused bitmap, so scrolling only paints the strip that was exposed.

---

[ref16]: https://example.com/ref/16

## 17. Section about topic 17

Text is shaped with DirectWrite and painted band by band into a reused bitmap, so scrolling only paints the strip that was exposed. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/17) or the [reference link][ref17] for details.

### 17.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

19. Ordered list starting at 19
20. Next number
   1. Nested ordered
   2. Another

#### 17.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 17.1.1.1 Code

###### 17.1.1.1.1 Smallest heading

Selections are anchored to source offsets, which means they survive relayout, resizing and scrolling without drifting.

---

[ref17]: https://example.com/ref/17

## 18. Section about topic 18

Selections are anchored to source offsets, which means they survive relayout, resizing and scrolling without drifting. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/18) or the [reference link][ref18] for details.

### 18.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

20. Ordered list starting at 20
21. Next number
   1. Nested ordered
   2. Another

#### 18.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 18.1.1.1 Code

###### 18.1.1.1.1 Smallest heading

Every number in this project is measured on a clean runner and reported in MiB together with the raw byte count.

---

[ref18]: https://example.com/ref/18

## 19. Section about topic 19

Every number in this project is measured on a clean runner and reported in MiB together with the raw byte count. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/19) or the [reference link][ref19] for details.

### 19.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

21. Ordered list starting at 21
22. Next number
   1. Nested ordered
   2. Another

#### 19.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 19.1.1.1 Code

###### 19.1.1.1.1 Smallest heading

YAMDV is a small, fast Markdown viewer for Windows. It opens a file, lays out only what you can see, and keeps memory low even for very large documents.

---

[ref19]: https://example.com/ref/19

## 20. Section about topic 20

YAMDV is a small, fast Markdown viewer for Windows. It opens a file, lays out only what you can see, and keeps memory low even for very large documents. It uses **strong text**, *emphasis*, ***both***, ~~strikethrough~~ and `inline code` like `cargo xtask memprobe`.

See the [project page](https://example.com/yamdv/20) or the [reference link][ref20] for details.

### 20.1 Lists

- First item with a short sentence.
- Second item
  - Nested item one
  - Nested item two
    - Third level
- Back to the first level

22. Ordered list starting at 22
23. Next number
   1. Nested ordered
   2. Another

#### 20.1.1 Quotes

> A quotation that spans a line.
> It continues here.
>
> > A nested quotation inside it.

##### 20.1.1.1 Code

###### 20.1.1.1.1 Smallest heading

The reader is built around a two-level document model: a top-level block index built in one pass, and detail trees that are parsed lazily when a block becomes visible.

---

[ref20]: https://example.com/ref/20

## Placeholders

| Feature | State |
|---|---|
| Tables | Phase 003 |
| Math | Phase 003 |

![Screenshot of the window](shot.png)

Inline math $e^{i\pi}+1=0$ and a footnote.[^note]

$$
\int_0^1 x^2\,dx = \frac13
$$

[^note]: Footnotes are rendered as a section in Phase 003.
