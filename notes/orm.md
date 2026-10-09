# Plain classes as database tables (card orm, 2026-10-08)

User decisions (notes/decisions.md "ORM"): plain classes, no base class or annotations, only registered; transactions
optional; filters work for any warp expression; a smart lazy-loading default chosen by us; migrations as proposed.

```warp
class Person{name: text; age: int}
people: [Person] = database.people     // registers: the table people holds Persons
people.add(Person("Bo", 30))           // INSERT
grown = people where it.age > 20       // SELECT … WHERE age > 20
bo = grown#1
bo.age += 1                            // UPDATE people SET age = 31 WHERE id = bo.id
```

## Registration
- `name: [C] = database.t` with C a class of the program registers table t for C. `database.t` without a class list
  stays what it is today: one value under the key t of the key-value store (lowering/stored_values.rs).
- `indexedDB.t` is the same (the alias of `database`).
- `stored people: [Person]` is the short form of `people: [Person] = database.people` (user, 2026-10-08).
- The schema comes from C's layout:
  - one column per field: int → INTEGER, float and quantities → REAL in the field's unit, text → TEXT, bool → INTEGER
    0/1;
  - an implicit `id` (INTEGER PRIMARY KEY) unless C has a field id;
  - a field of a registered class → a foreign key (its id), loaded lazily;
  - a list field of a registered class → one-to-many: the other table's rows pointing back, a query, not a column.
- Where it lives: natively `<program>.database.sqlite` beside the program (`:memory:` for code without a file, like
  database.json); in the browser IndexedDB, one object store per table, behind the same list API.

## The table is a list
Everything a list does works on a table: `count`, `#i`, `for p in people`, `where`, `map`, `add`/`+=`, `remove`.
Its elements are ordinary instances of C. An instance read from a table remembers its row (`id`), so a field change is
written through: `bo.age += 1` is an UPDATE of that row. `save bo` (a statement, its value bo) writes every column of
bo's row again, so it changes nothing after written-through changes; an instance without a row is an error there
("add it to people first"). Supervisor default, 2026-10-08, decisions.md.

## Filters: any warp expression
- `people where it.age > 20 and it.name.starts_with("B")`: the parts SQL has (comparisons, and/or/not, arithmetic,
  text concatenation, LIKE for starts_with/ends_with/contains, IN for `in [..]`) translate to SQL at compile time.
  Main-level variables become bound parameters.
- Any other pure warp function inside a filter (`people where is_prime(it.age)`) is registered on the connection as an
  SQLite application function (sqlite3_create_function_v2): `WHERE warp_fn_3(age)`. SQLite calls back into the
  running module, so every filter runs inside the query with SQL's own index use for the translated parts.
- An impure function in a filter is warned about at compile time (a query may run it any number of times).
- Backends without application functions (IndexedDB) push down what they can (key/index ranges) and filter the rest
  in memory: same results, the speed differs.
- Bare field names (P223, user): `people where age > 20` is `it.age > 20` when the element class has the field and no
  variable age is in scope; a variable in scope wins with a warning. Works on any list declared of a class
  (`people: [Person]`), table or not (comprehensions.rs Lists, fields_of_it).

## Loading: the smart default
- A table and every `where`/`sorted by`/`#a..b` on it is a **query**, not loaded. A query loads when the program reads
  elements: iteration streams in pages (default 100 rows), `#i` loads one row, `count` is SELECT COUNT(*), `exists`
  is LIMIT 1.
- Within one run each row is one instance (an identity map by id), so two queries giving Bo give the same Bo, and a
  change to one is seen in the other.
- Foreign keys load on first access (`bo.team.name`); one-to-many lists are queries too. A loop reading the same
  foreign key of every row (`for p in people { print p.team.name }`) is batched: the first access loads the teams of
  the whole page (IN (…)), which avoids N+1 queries without any annotation.
- Keywords to tune it (eager, page size) come later (user).

## Writes and transactions
- Default autocommit: each add/remove/field change is its own statement.
- `transaction { … }` (optional) is BEGIN … COMMIT, ROLLBACK when the block fails; it also batches.

## Migrations: stored schema vs class layout, at registration
| change | what happens |
|---|---|
| a field added | ALTER TABLE ADD COLUMN, the field's default value (else ø/NULL) |
| a field removed | the column stays, a warning names it (data is never dropped silently) |
| a lossless type or unit change (int → float, km → m) | converted in place (UPDATE … SET d = d * 1000) |
| a lossy change (float → int, text → int) | a loud error naming the column and both types |
| a rename | the field's meta `@was: old_name` renames the column (RENAME COLUMN); without it, it reads as add + remove |

## How step 1 works
- lowering/database_tables.rs (a source pass before class_methods): the registered class gets `id: int = 0`; the
  registration becomes `[Person(row#2, row#3, row#1) for row in std_io("table", "open", [t, schema, file])]`;
  `people.add(p)` adds and then sets `p.id` from `std_io("table", "insert", …)`; after each `v.f op= e` of a column f:
  `if v is C and v.id > 0 { std_io("table", "update", …) }`.
- src/database.rs: the SQLite C API through libloading (the system's libsqlite3, ffi/link.rs get_or_load_library), one
  connection per file and thread, `:memory:` for inline code. An added column takes the field's default, else the
  type's zero. A changed column type: see step 7. Browser: web/playground/host-files.js `table` (step 6).

## How filters work (steps 2 and 3, card orm-filters)
- comprehensions::lower_where (early, before database_tables::lower) asks database_tables::queried for a subject that is
  a registered table: `people where c` becomes
  `(table·ids·N = std_io("table", "select", [t, sql, [values…], file]); people where it.id in table·ids·N)`.
  The list keeps its instances, so a filtered row is the same instance (`bo.age += 1` shows in `people`).
- SQL keeps: comparisons, and/or, `+ - *` of numeric columns and number literals, `it.f` as its column, literals and
  variables as `?` values. Not `/` (warp's division is exact) and not `+` of texts or of unknown types.
- Every other part is `warp_call('table·call·N', id, columns…, values…)`: a generated function
  `table·call·N(table·arguments: any) := …` with `it.f` as its column's argument, `it` as the row's instance and the
  filter's variables as the values after the row. database.rs select registers warp_call for the query only
  (sqlite3_create_function_v2, user data the callback), host.rs queried_rows calls the export through route_value. A
  function that fails fails the query with its own trap (`raise "boom"` gives "boom").
- A function with side effects (State, IO, FFI, Async, Eval of effects.rs) in a table filter is a compile-time warning
  (card orm-filter, `effectful_calls`); it still runs, once per row in id order, as the in-memory filter does.
- Still loaded whole at registration: the query only picks ids. count/#i/paging and the identity map are step 2's rest.
- An element changed without the write-through form `v.f op= e` (`people#1.age = 5`) leaves its row stale, and a
  query of the table then disagrees with the list.

## How relations work (step 4; card orm)
- database_tables.rs with_relations: a field whose type is another registered class (`team: Team`) is a foreign key,
  the column `team` INTEGER holding the row's id; a list field of one (`players: [Person]`) is one-to-many, no column:
  the rows of people whose field of class Team points back (an error when Person has none).
- Opening people builds `team` as the row of teams with that id (`[r for r in teams if r.id == row#3]#1`), so a related
  row is the same instance as in its table. teams must be registered before people (a compile error otherwise).
  A one-to-many field is lazy: database_tables.rs with_member_getters makes `players: [Person]` the getter
  `players := { global people; [m for m in people if m.team.id == id] }`, a query at each read, so it is no
  constructor argument (`Team("Red")`) and a moved row (`bo.team = blue`) shows in both teams at once (card orm-moved).
- `people.add(p)` stores `p.team.id` (an instance without a row raises "… add it to its table first"); `bo.team = blue`
  writes blue's id through.
- In a filter, `it.team` is no column for SQL (an id vs an instance): the part goes through a query's function, which
  builds the instance from the id.
- Objects now point to each other, so a value read back from wasm can be cyclic: both readers mark a node met again
  inside itself as `…` (wasm_reader.rs, reader.js CYCLE_MARK). Printing one inside wasm (`print team`) still exhausts
  the call stack (card cyclic-print).
- Gaps: a row pointing to a deleted row, or the 0 of a column added for a foreign key, fails the open (index out of
  range; card orm-dangling). The getter scans the loaded list; natively it could be a SELECT once tables are queries.
- Sample: samples/orm.warp (native and playground) has no relations yet: adding `team` to its Person would hit
  orm-dangling on databases the sample already wrote.

## Steps
1. **Prototype, native, eager** (done: lowering/database_tables.rs, src/database.rs, tests/control/test_database_tables.rs):
   - registration, schema and the implicit id;
   - add and field updates written through;
   - migrations for added/removed columns;
   - the table loaded whole at registration, filters in memory.
2. Queries instead of loading: SQL translation of filters (done, card orm-filters), count/#i/paging, the identity map.
3. Application functions for the rest of a filter (done, card orm-filters: warp_call into the module).
4. Foreign keys and one-to-many (done, card orm; one-to-many lazy as getters), batched lazy loading of tables.
5. `transaction { }`.
6. IndexedDB backend in the browser (async underneath: the page's host keeps a loaded mirror per table, like the
   key-value store). Done simply (branch orm-updates): web/playground/host-files.js `table` keeps each table as
   one value `table <file> <name>` = {types, rows: [{id, column…}]} of the `database[k]` store, so it is loaded
   before the run and written back on every change. The browser-built compiler keeps a filter as the comprehension
   over the rows (database_tables::queried is native-only), since the browser has no SQL. One value per table rewrites
   the whole table on each change: fine for samples, slow for big tables (then one object store per table).
7. Unit and type conversions in migrations, `@was` renames. Type conversions done (branch orm-updates):
   - database.rs `converted`: INTEGER → REAL → TEXT converts forward (rename the column aside, add it with the new
     type, `UPDATE … SET c = CAST(old AS type)`, drop the old one, in a SAVEPOINT); a fresh column because SQLite's
     affinity would turn the values back. Any other change (REAL → INTEGER, TEXT → INTEGER) fails the open: "its values
     would lose data". host-files.js `convertColumn` does the same to the stored rows (a real's text keeps ".0").
   - Open: unit changes (km → m) wait for runtime units (a unit type is no column yet, notes/units_runtime.md); `@was`.
   - A field named size/count/length reads as the builtin count off a typed list element (card field-named-size).
