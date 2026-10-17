# Math

Inline formulas sit on the baseline: the energy $E = mc^2$, the ratio $\frac{a}{b}$, a root
$\sqrt{x^2 + y^2}$ and the sum $\sum_{i=1}^{n} i = \frac{n(n+1)}{2}$ inside running text.
Primes $f'(x)$ and $g''(t)$, dots $1, 2, \ldots, n$ and $a + \cdots + z$.

## Fractions and binomials

$$\frac{1}{1 + \frac{1}{x}} \qquad \dfrac{a}{b} \quad \tfrac{a}{b} \quad \binom{n}{k}$$

## Scripts, roots and big operators

$$x_i^2 + y_{j,k}^{n+1} \qquad \sqrt[3]{8} = 2 \qquad \sqrt{\frac{a}{b}}$$

$$\sum_{k=0}^{\infty} \frac{x^k}{k!} = e^x \qquad \prod_{p} \frac{1}{1 - p^{-s}} \qquad \int_0^1 x\,dx = \frac{1}{2} \qquad \oint_C \mathbf{F} \cdot d\mathbf{r}$$

$$\lim_{n \to \infty} \left(1 + \frac{1}{n}\right)^n = e \qquad \max_{x \in S} f(x)$$

## Delimiters

$$\left( \frac{a}{b} \right) \left[ \frac{\frac{1}{2}}{3} \right] \left\{ x \mid x > 0 \right\} \left| \frac{p}{q} \right|$$

$$\big( \Big( \bigg( \Bigg( x \Bigg) \bigg) \Big) \big)$$

## Environments

$$\begin{pmatrix} a & b \\ c & d \end{pmatrix} \begin{bmatrix} 1 & 0 \\ 0 & 1 \end{bmatrix} \begin{vmatrix} x & y \\ z & w \end{vmatrix} \begin{matrix} 1 & 2 \\ 3 & 4 \end{matrix}$$

$$f(x) = \begin{cases} x^2 & x \ge 0 \\ -x & x < 0 \end{cases}$$

$$\begin{aligned} (a+b)^2 &= a^2 + 2ab + b^2 \\ &= b^2 + 2ab + a^2 \end{aligned}$$

## Fonts, accents, Greek, symbols

$$\mathbb{R} \; \mathbb{N} \; \mathcal{L} \; \mathbf{v} \; \mathit{text} \qquad \hat{x} \; \bar{y} \; \vec{v} \; \dot{a} \; \ddot{b} \; \tilde{n}$$

$$\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \lambda \mu \pi \sigma \phi \omega \quad \Gamma \Delta \Theta \Lambda \Sigma \Phi \Omega$$

$$a \le b \ge c \ne d \approx e \equiv f \sim g \subset h \subseteq i \in j \notin k \quad x \to y \Rightarrow z \leftrightarrow w \mapsto v \quad \forall \exists \nabla \partial \infty \pm \times \div$$

## Spacing and color

$$a\,b\;c\quad d\qquad e \! f \qquad {\color{red} x^2} + {\color{#2563eb} y^2}$$

```math
\overline{AB} + \underline{CD} = \sin\theta + \cos\theta + \log x
```

## Errors

Unknown command inline: $\foo{x}$ keeps its source.

$$\frac{a}{$$

$$\begin{pmatrix} a & b \end{bmatrix}$$
