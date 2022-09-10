
# Tests
## Unit tests
```bash
cargo test

```
## Integration tests
```bash
cargo run -- data/transactions_complete.csv > /tmp/output.log
```

With the following input :

|type       |client|tx  |amount  |
|-----------|------|----|--------|
|deposit    | 1    | 1  | 1.0    |
|deposit    | 2    | 2  | 2.0    |
|deposit    | 1    | 3  | 2.0    |
|withdrawal | 1    | 4  | 1.5    |
|withdrawal | 2    | 5  | 3.0    |
|deposit    | 3    | 6  | 3.5    |
|deposit    | 3    | 7  | 10.0   |
|dispute    | 3    | 6  |        |
|deposit    | 4    | 8  | 13.5   |
|deposit    | 4    | 9  | 110.0  |
|resolve    | 4    | 8  |        |
|dispute    | 4    | 8  |        |
|resolve    | 4    | 8  |        |
|deposit    | 5    | 10 | 113.5  |
|deposit    | 5    | 11 | 1110.0 |
|chargeback | 5    | 10 |        |
|dispute    | 5    | 10 |        |
|chargeback | 5    | 10 |        |

Expect the following :

|client |available|held|total  |locked|
|-------|---------|----|-------|------|
|1      |1.5      |0.0 |1.5    |false |
|2      |2.0      |0.0 |2.0    |false |
|3      |10.0     |3.5 |13.5   |false |
|4      |123.5    |0.0 |123.5  |false |
|5      |1110.0   |0.0 |1110.0 |true  |
                               
# Remarks
Error logs are logged to stderr.
