const id = 123;
const query = `SELECT * FROM orders WHERE id = ${id}`;
const db = { query: (q: string) => {} };
db.query(query);
