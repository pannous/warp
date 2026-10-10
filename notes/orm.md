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
`people.remove(bo)` (card table-remove) is a DELETE of bo's row (`std_io("table", "delete", …)`, IndexedDB too), bo
keeping its fields with id 0; the lazy `people·remove` drops bo from the loaded list or the instances met before.
`for p in people where p.age > 18 { … }` walks `people where it.age > 18`, so the filter stays SQL (card for-where).

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

### How loading works now (branch orm-updates)
- Registration migrates (`std_io("table", "migrate", …)`) and loads no rows. database_tables.rs `opened` generates per
  table `people·load()` (rows via `std_io("table", "rows", …)` on the first call, `people·loaded`), `people·count()`
  (SELECT COUNT(*) until loaded; a table with a required foreign key loads, since such rows can be left out),
  `people·add(p)` (onto the list when loaded, else onto `people·met`), `people·at(i)` (until loaded the one row
  `std_io("table", "page", […, i, 1])`, SELECT … LIMIT 1 OFFSET i-1, its instance kept in `people·met` by
  `people·kept(row)`; a required foreign key, a position below 1 or past the end loads), `people·streamed(i)` (the
  element of a loop: until loaded from the page of 100 rows holding i, cached in `people·page`/`people·start`) and `people·reset()`.
- `with_lazy_reads` (last in database_tables::lower) turns each read `people` into `people·load()`, `count(people)` and
  `people.count` into `people·count()`, `people.add(p)` into `people·add(p)`, `people#i` into `people·at(i)`, `for p in people {…}` into
  `p·end = people·count(); for p·position in 1 to p·end { p: Person = people·streamed(p·position); … }` (break and
  continue as in any range loop; rows the body adds are not walked); registrations, `global people`, assigned
  lists and field names stay, and a table's own lazy functions keep its list. One-to-many getters select natively (`people·found`), in the browser they read `people·load()`.
- Identity: the load builds each row's instance unless `people·met` holds one with its id, so an instance added before
  the load is the loaded row (test an_instance_added_before_loading_is_the_loaded_row).
- A route reopening the table (`users = database.users`, serve.rs) is `users = users·reset()`: the next read loads anew.
- Observed natively by `database::rows_read()` (tests a_table_loads_its_rows_only_when_read,
  an_element_of_a_table_loads_one_row, iterating_a_table_reads_it_in_pages).
- The identity map `people·met = {}` is keyed by id (`people·met[id]`, `id in people·met`), a global hash table
  (map_backend.rs find_typed_map_globals, card int-map): loading, adding and reading n rows is linear, 10 000 rows in
  ~0.5 s with their inserts (test ten_thousand_rows_load_in_linear_time; a list searched by id took 11.6 s for 2000).
  The INSERT runs before the add so the instance has its id as its key. A rollback's restore finds rows by id in a
  local map too. The map holds every loaded and added instance, so a foreign key finds its row by `teams·of(id)`
  (`teams.table_instance_of(id)` until with_lazy_reads; ø when no row has it) and a loaded table's filter maps its
  selected ids through it (card orm-linear, tests/control/test_table_linear_lookups.rs). A key to the table's own class
  still walks the list read so far (`teams·of` there would load the table again).
- Linear loads also needed two list cursor fixes (list_ops.rs): an append (list_extend) keeps the index cursor and the
  count of other lists, so a comprehension over the rows read walks on; a short index or count (`row#4`) leaves the
  cursor where it is. A conditional typed-list append (`if c { out += [x] }`) runs as a statement, not as a Node copy.
- A filter `people where c` is `people.table·found(sql, parameters)` until with_lazy_reads makes it `people·found(…)`:
  `std_io("table", "select", [table, schema, file, sql, parameters])` gives the kept rows whole, each made the one
  instance of its row by `people·kept` (no load); a loaded table, or one with a required foreign key, keeps its loaded
  instances of those ids (test a_filter_reads_only_the_rows_it_keeps). A served route answering one answers [] when
  empty (serve.rs answers_a_list, database_tables::is_filter_query).
- A class method reads and filters tables as a function does: with_lazy_reads and lower_where enter class bodies (a
  `Node::Type` is no child of map_children), only method bodies, and a field named like a table stays the field.
- Not yet: comprehensions over a table (they load it), the IN (…) batching. An empty list must be `parse("[]")` (ø): a built `[]` List node with Space
  separator types `xs += [x]` as int + list.

## Writes and transactions
- Default autocommit: each add/remove/field change is its own statement.
- `transaction { … }` (optional) is BEGIN … COMMIT, ROLLBACK when the block fails; it also batches. Done (branch
  orm-transaction, database_tables.rs in_transaction): `std_io("table", "begin"/"commit"/"rollback", [file])`, the block
  under `try … catch`; a failure rolls back, then each open table's `people·restore()` gives the instances the program
  holds their rows' values again (an instance whose row is gone gets id 0) and loads anew, then the failure is raised
  again. Its value is the block's. The browser's store snapshots the file's tables at begin (host-files.js).
  Sample: samples/orm_transaction.warp.

## Migrations: stored schema vs class layout, at registration
| change | what happens |
|---|---|
| a field added | ALTER TABLE ADD COLUMN, the field's default value (else ø/NULL) |
| a field removed | the column stays, a warning names it (data is never dropped silently) |
| a lossless type change (int → float) | converted in place (UPDATE … SET c = CAST(old AS type)) |
| a unit change within its quantity (km → m) | only the column's declared type changes: a unit field holds SI amounts |
| plain numbers given a unit (int → km) | read as that unit (UPDATE … SET d = old * 1000), a warning names the unit |
| another quantity (km → kg), a unit dropped (km → float) | a loud error naming the column and both types |
| a lossy change (float → int, text → int) | a loud error naming the column and both types |
| a rename | the field's annotation `@was(old_name) name: text` renames the column (RENAME COLUMN); without it, it reads as add + remove |

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
  `==` and `!=` are SQL's `IS` and `IS NOT`, so `it.email == ø` finds the NULLs as in warp.
- An optional scalar field (`email: text?`, card orm-optional) is a nullable column: rows without a value (and rows
  from before an added column) hold NULL and read as ø. `text` → `text?` keeps the column (the browser store too).
- Every other part is `warp_call('table·call·N', id, columns…, values…)`: a generated function
  `table·call·N(table·arguments: any) := …` with `it.f` as its column's argument, `it` as the row's instance and the
  filter's variables as the values after the row. database.rs select registers warp_call for the query only
  (sqlite3_create_function_v2, user data the callback), host.rs queried_rows calls the export through route_value. A
  function that fails fails the query with its own trap (`raise "boom"` gives "boom").
- A function with side effects (State, IO, FFI, Async, Eval of effects.rs) in a table filter is a compile-time warning
  (card orm-filter, `effectful_calls`); it still runs, once per row in id order, as the in-memory filter does.
- An element changed in place (`people#1.age = 5`, `(people where …)#1.age += 1`) is bound first,
  `people·element = people#1; people·element.age = 5`, so it is written through as a variable's change is
  (database_tables.rs with_bound_element; it was "people·at() gives a copy").

## How relations work (step 4; card orm)
- database_tables.rs with_relations: a field whose type is another registered class (`team: Team`) is a foreign key,
  the column `team` INTEGER holding the row's id; a list field of one (`players: [Person]`) is one-to-many, no column:
  the rows of people whose field of class Team points back (an error when Person has none).
- Opening people builds `team` as the row of teams with that id (`[r for r in teams if r.id == row#3]#1`), so a related
  row is the same instance as in its table. teams must be registered before people (a compile error otherwise).
  A one-to-many field is lazy: database_tables.rs with_member_getters makes `players: [Person]` the getter
  `players := { people·found("\"team\" IS ?", [id]) }` natively, the SELECT of the rows pointing back (the people
  table is not loaded; test a_list_field_reads_only_the_rows_pointing_back), and in the browser the scan
  `[m for m in people·load() if m.team.id == id]`: a query at each read, so it is no
  constructor argument (`Team("Red")`) and a moved row (`bo.team = blue`) shows in both teams at once (card orm-moved).
- `people.add(p)` stores `p.team.id` (an instance without a row raises "… add it to its table first"); `bo.team = blue`
  writes blue's id through.
- `red.players.add(p)` (anywhere, `d.team.players.add(d)` too; cards orm-nested, orm-list-add) sets `p.team = red`:
  written to p's row, or p inserted into people when it has none; no duplicate row (database_tables.rs
  added_to_members).
- In a filter, `it.team` is no column for SQL (an id vs an instance): the part goes through a query's function, which
  builds the instance from the id.
- Objects now point to each other, so a value read back from wasm can be cyclic: both readers mark a node met again
  inside itself as `…` (wasm_reader.rs, reader.js CYCLE_MARK). Printing one inside wasm (`print team`) still exhausts
  the call stack (card cyclic-print).
- A key without its row (a deleted row, or the 0 of a column added for a foreign key; card orm-dangling): a required
  key (`team: Team`) leaves its row out of the opened list with a runtime warning naming the row and suggesting
  `team: Team?`; the row stays in the database. An optional key (`team: Team?`) reads ø and is stored as 0
  (database_tables.rs opened, referenced, key_id).
- Gaps: a loop reading `t.players` of every team runs a SELECT per team (no IN (…) batching yet); a required back key
  (`team: Team`) makes `people·found` load the table, as any filter of such a table does.
- Sample: samples/orm.warp (native and playground) has `team: Team?` and `players: [Person]`: optional, so databases
  the sample wrote before the column read ø (card orm-dangling); `climbers.players.add(bo)` writes bo.team through.
- An add or field write in a one-statement block (`if … { teams.add(t) }`) is lowered like a statement of its own.

## Steps
1. **Prototype, native, eager** (done: lowering/database_tables.rs, src/database.rs, tests/control/test_database_tables.rs):
   - registration, schema and the implicit id;
   - add and field updates written through;
   - migrations for added/removed columns;
   - the table loaded whole at registration, filters in memory.
2. Queries instead of loading: SQL translation of filters (done, card orm-filters), lazy load + count + add without
   loading + identity of added instances (done, orm-updates), #i/paging.
3. Application functions for the rest of a filter (done, card orm-filters: warp_call into the module).
4. Foreign keys and one-to-many (done, card orm; one-to-many lazy as getters), batched lazy loading of tables.
5. `transaction { }` (done, branch orm-transaction).
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
   - `@was(nick) name: text` (an annotation on the field's name; class_methods::fields_marked) sends the column's old
     name as a 4th schema entry: database.rs renames it (RENAME COLUMN) when the table has the old column and not the
     new one, host-files.js `renameColumn` moves the stored values. `@was: nick` inside a class body is Ruby's `self.was`.
   - Unit fields (card unit-fields): `class Run{distance: km}` (static_units/unit_fields.rs) holds the SI amount like
     any quantity; the column is `NUMERIC km` (NUMERIC keeps a whole SI amount an integer, as the program holds it; REAL
     gave 5000.0, which an exact `total = 0 km` refused: "not an int"). database.rs `conversion_factor` decides the
     migrations above. The playground's store follows the same rules (card browser-unit-migrations): the schema
     entry of a unit field carries its quantity and SI amount per unit (`[distance km ø ø m 1000]`, the 4th entry ø
     unless renamed; units::static_units::unit_type), the store keeps each unit column's quantity (`quantities`).
     An optional unit field (`climb: m?`, card unit-field) is a nullable `NUMERIC m` column declared `number?`; ø has
     no dimension, so `Run(5 km, ø)` and `r.climb == ø` pass static units. Sample: samples/orm_units.warp.
   - A field named size/count/length reads as the builtin count off a typed list element (card field-named-size).
