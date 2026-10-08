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
written through: `bo.age += 1` is an UPDATE of that row.

## Filters: any warp expression
- `people where it.age > 20 and it.name.starts_with("B")`: the parts SQL has (comparisons, and/or/not, arithmetic,
  text concatenation, LIKE for starts_with/ends_with/contains, IN for `in [..]`) translate to SQL at compile time.
  Main-level variables become bound parameters.
- Any other pure warp function inside a filter (`people where is_prime(it.age)`) is registered on the connection as an
  SQLite application function (sqlite3_create_function_v2): `WHERE warp_fn_3(age)`. SQLite calls back into the
  running module, so every filter runs inside the query with SQL's own index use for the translated parts.
- An impure function in a filter is a loud error (a query may run it any number of times).
- Backends without application functions (IndexedDB) push down what they can (key/index ranges) and filter the rest
  in memory: same results, the speed differs.
- Bare field names (`people where age > 20`, as SQL writes it) are open: warp's `where` demands `it.age` today, loudly
  (question for the Interviewer).

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

## Steps
1. **Prototype, native, eager** (this branch):
   - registration, schema and the implicit id;
   - add and field updates written through;
   - migrations for added/removed columns;
   - the table loaded whole at registration, filters in memory.
2. Queries instead of loading: SQL translation of filters, count/#i/paging, the identity map.
3. Application functions for the rest of a filter (callback into the module from sqlite3_create_function_v2).
4. Foreign keys and one-to-many, batched lazy loading.
5. `transaction { }`.
6. IndexedDB backend in the browser (async underneath: the page's host keeps a loaded mirror per table, like the
   key-value store).
7. Unit and type conversions in migrations, `@was` renames.
