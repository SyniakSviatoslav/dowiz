const { Pool } = require('pg');

async function testConnection(connectionString, description) {
  const pool = new Pool({
    connectionString,
    max: 1,
    ssl: { rejectUnauthorized: false }
  });

  try {
    const client = await pool.connect();
    console.log(`✅ ${description} connected`);
    await client.query('SELECT 1');
    console.log(`✅ ${description} query worked`);
    await client.release();
  } catch (e) {
    console.log(`❌ ${description} failed: ${e.message}`);
  } finally {
    await pool.end();
  }
}

// Test 1: Operational pool (should work)
testConnection(
  'postgresql://REDACTED:REDACTED@REDACTED-supabase-host/db',
  'Operational pool (6543)'
);

// Test 2: Session pool with operational user (testing if user works on port 5432)
testConnection(
  'postgresql://REDACTED:REDACTED@REDACTED-supabase-host/db',
  'Operational user on port 5432'
);

// Test 3: Actual session pool string (should fail)
testConnection(
  'postgresql://REDACTED:REDACTED@REDACTED-supabase-host/db',
  'Session pool (actual credentials)'
);