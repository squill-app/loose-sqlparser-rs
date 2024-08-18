use criterion::{criterion_group, criterion_main, Criterion};

const SQL: &str = r#"
/** 
 * This is a recursive query to find the first 3 levels of employees under the employee with id 1.
 * The query will return the employee_id, first_name, manager_id, and level of the employee.
 * The query will only return employees with status 'active'.
 */
WITH emp_data AS (
  (
    SELECT employee_id, first_name, manager_id, 1 AS level, status
      FROM employee
      WHERE employee_id = 1
  )
  UNION ALL
  (
    SELECT this.employee_id, this.first_name, this.manager_id, prior.level + 1
    FROM emp_data prior
    INNER JOIN employee this ON this.manager_id = prior.employee_id
  )
) SELECT e.employee_id, e.first_name, e.manager_id, e.level
  FROM emp_data e WHERE e.level <=3 AND e.status = 'active'
  ORDER BY e.level
"#;

fn sqlparser(sql: &str) {
    let dialect = sqlparser::dialect::GenericDialect {};
    let res = sqlparser::parser::Parser::parse_sql(&dialect, sql);
    assert!(res.is_ok());
}

fn loose_sqlparser(sql: &str) {
    let _ = loose_sqlparser::loose_sqlparse(sql);
}

fn bench_parsers(c: &mut Criterion) {
    let mut group = c.benchmark_group("Parsers");
    group.bench_function("sqlparser", |b| b.iter(|| sqlparser(SQL)));
    group.bench_function("loose_sqlparser", |b| b.iter(|| loose_sqlparser(SQL)));
    group.finish();
}

criterion_group!(benches, bench_parsers);
criterion_main!(benches);
