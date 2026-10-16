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

## Feature tour

> [!NOTE]
> This section exercises the Phase 003 renderer: callouts, tables, code, images and footnotes.[^tour]

| Feature | Status | Notes |
|:--------|:------:|------:|
| Tables | done | auto layout |
| Code | done | 30 languages |
| Callouts | done | GitHub + Obsidian |
| Footnotes | done | numbered by first use |

```rust
fn main() {
    let names = ["alpha", "beta", "gamma"];
    for (i, n) in names.iter().enumerate() {
        println!("{i}: {n}");
    }
}
```

```python
def fib(n: int) -> int:
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a
```

```json
{ "name": "yamdv", "offline": true, "version": 3 }
```

![Gradient sample](typical-assets/one.png)

![Checker sample](typical-assets/two.png)

[^tour]: The tour is original content written for the YAMDV corpus.

### Notes 1 

Scroll paragraph table budget render offline budget native scroll memory budget budget image paragraph scroll paragraph measure image offline render window viewer document offline native budget image callout image footnote paragraph table layout viewer paragraph budget memory native callout document callout measure image table callout scroll heading offline native window heading layout memory native document layout render budget budget render table render callout paragraph offline document callout callout layout viewer.

Viewer image heading native measure scroll viewer document offline render layout viewer memory callout document native paragraph viewer heading heading table paragraph image image footnote measure image layout measure native callout scroll document callout native measure document heading offline callout heading offline image paragraph viewer heading footnote table table native budget offline viewer offline table native footnote document heading window table window heading table native document image layout offline paragraph.


### Notes 2 

Document measure scroll native scroll heading window callout layout layout heading heading scroll footnote window render heading memory footnote native scroll layout viewer measure memory heading window native heading render table paragraph callout document measure native footnote table callout document callout callout viewer callout paragraph memory offline budget measure callout scroll viewer footnote measure document heading scroll render document layout scroll viewer viewer measure memory callout viewer offline budget layout.

Window measure document scroll offline measure callout viewer layout heading paragraph native budget viewer table scroll memory layout layout window scroll native paragraph offline budget image viewer native scroll native measure measure callout viewer budget heading offline viewer paragraph viewer layout viewer render budget viewer render measure measure budget heading window heading render table image image document table native memory heading callout layout paragraph offline image render window viewer table.


### Notes 3 

Footnote image viewer callout viewer table budget heading callout callout footnote offline scroll memory native render callout scroll callout paragraph offline heading table native layout footnote layout measure image layout heading layout offline budget paragraph scroll image viewer measure render layout image window offline heading layout offline layout budget document document render viewer measure measure scroll layout layout viewer heading window render scroll window scroll footnote image native table image.

Table callout render image measure scroll callout window callout measure budget paragraph image paragraph window layout budget budget measure footnote window paragraph native memory paragraph measure heading scroll document callout native memory document offline native budget image memory window table scroll heading budget paragraph callout budget memory footnote offline budget render image viewer footnote scroll scroll render memory native scroll memory native paragraph window viewer native window viewer heading window.


### Notes 4 

Callout render render layout render native document viewer table footnote heading offline offline heading heading callout image budget render memory budget image paragraph heading layout native render callout layout footnote measure native layout measure table table footnote document native layout heading measure layout budget measure table viewer document window paragraph window table footnote layout layout paragraph heading callout document window footnote budget document window render layout window image table layout.

Native native image viewer paragraph viewer budget measure native scroll measure table heading image footnote render table budget layout paragraph native layout layout layout window memory callout image paragraph paragraph image memory measure window window memory native table document offline footnote callout image heading document budget measure document budget offline memory offline layout scroll budget window measure footnote memory memory measure memory viewer measure footnote layout document paragraph paragraph footnote.


### Notes 5 

Render viewer offline table scroll measure render budget offline heading heading heading table paragraph render viewer render heading memory render memory callout scroll budget heading layout viewer callout render memory window image budget budget render callout memory budget document offline footnote footnote image footnote window footnote viewer native table table footnote measure table image scroll offline memory native table paragraph footnote memory window memory offline window image measure window offline.

Paragraph layout window footnote budget window viewer offline image footnote heading callout viewer viewer scroll image viewer image budget offline scroll scroll layout image budget memory window heading layout table layout viewer document native footnote document budget scroll native offline heading table heading render viewer callout render offline layout offline render offline window measure viewer budget viewer memory measure heading memory budget heading budget table viewer image document image render.


### Notes 6 

Document window callout layout measure image heading image window image table window table callout footnote scroll footnote budget table native window measure image budget viewer paragraph window offline footnote render layout heading scroll viewer viewer footnote footnote heading table offline render memory image layout heading document layout footnote render memory scroll viewer paragraph paragraph offline layout scroll document memory scroll measure callout measure heading memory footnote window render viewer layout.

Offline layout memory native render layout footnote image scroll layout budget table image footnote layout document footnote image memory layout offline footnote document render heading table memory budget render table callout render measure memory scroll table viewer heading scroll callout footnote render native memory heading window memory memory footnote callout table memory callout budget callout budget viewer document offline window layout offline paragraph document measure measure viewer budget viewer memory.


### Notes 7 

Memory native budget callout viewer table footnote memory document paragraph layout footnote document callout footnote render memory paragraph budget document image table window callout document footnote budget measure scroll table document document offline footnote table table document scroll measure offline offline paragraph measure paragraph offline window viewer offline memory footnote table table viewer budget window scroll offline native callout heading viewer layout footnote document native scroll budget callout native heading.

Viewer offline callout viewer window scroll paragraph callout measure heading paragraph native offline window viewer offline budget viewer footnote footnote measure measure callout table measure window document window render paragraph layout callout table footnote footnote native native footnote document measure paragraph heading paragraph measure viewer callout budget scroll footnote native offline heading layout budget paragraph native native layout callout render table viewer measure budget footnote memory document table window image.


### Notes 8 

Image heading viewer native memory viewer heading heading image document viewer paragraph callout native callout render budget scroll memory render measure layout layout offline document render callout native budget footnote native document viewer window scroll budget window paragraph paragraph window footnote image offline paragraph image viewer window window document memory paragraph paragraph viewer measure paragraph memory image layout callout image window offline native layout paragraph layout paragraph document paragraph image.

Table render memory offline table paragraph budget scroll render table budget layout heading budget offline table measure footnote callout footnote document footnote paragraph footnote image memory document window document window heading native memory paragraph viewer viewer callout window layout offline window layout image table measure native render footnote footnote heading paragraph memory heading footnote measure table heading table table heading document document native window layout measure scroll heading scroll document.


### Notes 9 

Callout native footnote paragraph budget heading window measure measure footnote viewer render callout callout document viewer scroll image image memory render table measure memory viewer budget layout callout image table offline document table measure table image footnote table layout budget paragraph heading scroll offline table render offline paragraph render memory heading callout document memory offline offline heading footnote table memory footnote heading layout measure image scroll budget paragraph document document.

Memory layout window render callout offline table image offline budget window budget native paragraph image memory measure callout document callout document layout render render footnote heading render offline heading budget callout layout heading callout scroll heading memory image render viewer measure offline measure measure scroll render window scroll budget image document table footnote measure scroll memory document heading paragraph table measure table heading measure memory footnote viewer callout document memory.


### Notes 10 

Budget memory window layout footnote layout measure image memory heading window viewer heading budget scroll image viewer scroll image table image scroll window document native table native viewer heading layout memory scroll document footnote window window scroll budget memory memory table scroll window measure image callout window document footnote table viewer render footnote budget offline window native measure callout budget scroll image measure budget table image budget window layout callout.

Heading paragraph heading layout table measure paragraph document budget paragraph footnote table budget viewer table memory render document document heading callout document offline callout table callout budget table window scroll document memory image layout heading window heading budget window image footnote memory callout table layout viewer budget memory window measure window layout layout layout image measure heading image native render budget native heading scroll native layout memory layout budget table.


### Notes 11 

Callout paragraph native table window native measure memory scroll scroll native table heading layout layout paragraph render scroll footnote heading layout heading paragraph window viewer budget native measure paragraph table footnote callout measure callout callout native document native image paragraph viewer viewer window callout offline memory paragraph measure memory table native viewer table memory layout window memory render budget callout document paragraph layout heading window native paragraph render render paragraph.

Layout offline render measure image footnote footnote native window offline render document callout window offline viewer heading measure heading table layout viewer memory image offline footnote heading scroll footnote footnote scroll offline table memory scroll window callout image layout image heading layout scroll document layout offline image offline heading scroll memory footnote measure native image memory measure callout document render window footnote measure measure layout document paragraph footnote measure paragraph.


### Notes 12 

Document measure window scroll table measure document layout render budget render budget scroll callout offline offline offline budget offline viewer measure budget offline viewer table document document budget budget document document paragraph document render image viewer layout memory scroll render window image measure measure paragraph window render document measure measure offline budget render table budget render callout heading image callout footnote heading offline heading document budget native budget native memory.

Table layout native budget callout budget memory heading viewer scroll viewer image scroll image window heading layout window measure footnote native callout callout heading image callout viewer table image table image native measure scroll paragraph scroll memory offline callout layout image budget budget image measure measure callout memory budget image viewer window offline memory layout paragraph image measure layout layout layout memory document paragraph callout callout paragraph window memory callout.


### Notes 13 

Budget heading footnote measure offline native offline heading heading image memory layout document memory render memory image scroll scroll heading scroll render heading document memory memory image offline measure document scroll paragraph native render native table memory document paragraph table memory memory layout footnote callout native viewer image native document measure memory footnote memory document table render layout callout image table scroll native layout layout offline window heading table window.

Scroll heading viewer layout heading render callout heading document offline budget footnote footnote window budget offline layout window table budget native image measure memory footnote image budget table table offline heading budget budget footnote offline budget render document window scroll layout window window native heading render table offline scroll heading offline viewer image window layout measure footnote table budget offline footnote callout render layout render render memory layout native render.


### Notes 14 

Heading offline memory viewer footnote viewer layout measure memory heading footnote document heading offline memory offline memory callout paragraph table footnote measure measure callout budget paragraph document document render scroll window heading footnote image scroll measure heading document heading table window document table render window image document footnote image measure callout memory layout table offline table footnote viewer footnote measure native budget measure budget memory document render render budget footnote.

Image window memory native paragraph native offline window memory scroll footnote layout document table memory measure layout offline heading footnote document budget viewer heading document window footnote budget document measure budget heading scroll render memory viewer table render table offline window viewer budget window heading layout window table scroll budget measure viewer offline scroll heading paragraph document scroll table table image paragraph footnote callout image viewer footnote image offline render.

