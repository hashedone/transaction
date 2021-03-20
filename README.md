## Running

Application runs as expected:

```
cargo run -- ./input_file.csv
```

However silently ignoring every error was very not nice for me, so I decided after all to do some simple error hanling, and at least log them. However I couldn't turn it on by default, as it might cause problems with automatic usage of tools (teoretically I could assume that noone uses stderr, but I just wanted to be sure), so to enable logging rejection reason, environment variable has to be set:

```
RUST_LOG=warn cargo run -- ./input_file.csv
```

Output csv is printed to stdout, and rejection reasons are on stderr, so they can be easly split. I know, that error messages are not the best, but I just wanted to have something (even for debugging), and didn't spend ages on it.

## Problems

They are actually mentioned in comments, but here I pointed my decisions I was not sure (or I was sure, but I just want explain).

### External crates

Obviously `csv` and `serde` for serialization. Also I included `anyhow` for easy error hanling. `log` and `pretty_env_logger` for sane configurable logging.

### Decision

There were some decisions to be done, which were not precisely described, here are most important:

* reasonable ppl doesn't perform money calculations on floats, and I try to be reasonable, so everything is done on fixed-point amount
* any transaction with tx, should have unique tx; This is actually documented, but there is nothing about what if it is not - I decided to reject such transaction
* no transactions may be performed on locked client; It might be very much wrong assumption but it seems like client which was charged back is just untrustfull
* only deposit transaction can be disputed; This again might be very invalid assumption, but disputing withdraw transaction might create ficional money on client acc which could be used, this just looks logically wrong
* transaction which doesn't parse are just rejected
* resolve and chargeback are "undisputing" transaction - not mentioned directly, but I think it is kind of obvious

## Validation

Some critical, easy to mess up things are unit-tested. However most of testing is done just by adding new client with some specific transaction flow to `./input/basic.csv`. Nothing fancy, but valid.


## Safety

No unsafe. Nope, nope. Just no. It was one day challange. Only reason to use unsafe is to do cutting edge optimisations, and to do so, I would need to profile, and to do this I would need to provide big dataset, and so on. I didn't have time for all of this, and additionally to make sure about it soundess. Just keeping things sane.
