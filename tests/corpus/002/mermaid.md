# Mermaid

## Flowchart: shapes and edges

```mermaid
flowchart LR
  %% every node shape of the core subset
  r[Rect] --> o(Round)
  o --- s([Stadium])
  s -.-> b[[Subroutine]]
  b ==> c[(Cylinder)]
  c -.- k((Circle))
  k === d{Rhombus}
  d --> h{{Hexagon falls back}}
```

## Flowchart: labels, chains, groups and styles

```mermaid
graph TD
  start([Start]) --> read[Read input]
  read --> ok{Valid?}
  ok -->|yes| work[[Process]]
  ok -- no --> fail((Error))
  fail -. retry .-> read
  work == store ==> db[(Database)]
  db --> a & b --> done(Done)
  click start callback "ignored"
  subgraph svc [Backend]
    work
    db
    subgraph inner [Workers]
      a[Worker A]
      b[Worker B]
    end
  end
  classDef warn fill:#fde68a,stroke:#b45309,color:#78350f
  class fail warn
  done:::warn
  style read fill:#dcfce7,stroke:#16a34a
```

## Flowchart: other directions

```mermaid
flowchart BT
  x1[Bottom] --> x2[Middle] --> x3[Top]
  x1 --> x3
```

```mermaid
graph RL
  p[Right] --> q[Left]
  q --> q
```

## Sequence diagram

```mermaid
sequenceDiagram
  title: Sign-in
  autonumber
  actor U as User
  participant B as Browser
  participant S as Server
  U->>B: Open page
  B->>+S: GET /login
  S-->>-B: 200 OK
  B->S: solid open
  B-->S: dotted open
  S-xB: solid cross
  S--xB: dotted cross
  B-)S: async
  S--)B: dotted async
  activate B
  B->>B: render
  deactivate B
  Note left of U: at home
  Note right of S: cached
  Note over B,S: TLS everywhere
  loop every minute
    B-)S: heartbeat
  end
  alt valid password
    S->>B: token
  else invalid
    S-xB: reject
  end
  opt remember me
    B->>S: store cookie
  end
  par mail
    S->>U: email
  and log
    S->>S: audit
  end
```

## Unsupported type

```mermaid
pie title Pets
  "Dogs" : 40
  "Cats" : 60
```

## Syntax error

```mermaid
flowchart TD
  A --> B
  B -->
```
