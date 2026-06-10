use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use jexpr::parse;

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    group.bench_function("identifier", |b| b.iter(|| parse(black_box("foo"))));

    group.bench_function("complex expression", |b| {
        b.iter(|| parse(black_box("(a + b([1, 2, 3]) * c)")))
    });

    group.bench_function("big expression", |b| {
         b.iter(|| {
             parse(black_box(
                 "users.filter((u) => u.age >= 18).map((u) => ({name: u.name, adult: true})).reduce((acc, u) => acc + u.name.length, 0)",
             ))
         })
     });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
