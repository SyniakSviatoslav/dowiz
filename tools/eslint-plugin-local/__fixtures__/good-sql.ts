const id = 123;
const query = "SELECT * FROM orders WHERE id = $1";
const db = { query: (q: string, params: any[]) => {} };
db.query(query, [id]);
