# tally

Summarize CSV expense records by category.

## Input

A CSV file with a header row and the columns `date` (YYYY-MM-DD), `category`,
`amount`, and optionally `note`. Category names are case-insensitive. A row
that cannot be read stops the run with a message naming its line.

## Usage

```text
python -m tally report FILE [--top N]
```

Prints one line per category, largest total first, then a `total` line:

```text
$ python -m tally report data/expenses.csv
rent           2500.00
groceries       277.30
utilities       178.55
transport        96.00
total          3051.85
```

`--top N` shows only the N largest categories; the total still covers all
records.

## Development

```text
python -m unittest discover -s tests -v
```

No dependencies beyond the standard library.
