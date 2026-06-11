const { Pool } = require('pg');

async function testConnection(connectionString, description) {
  const pool = new Pool({
    connectionString: connectionString,
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
  'postgresql://deliveryos_api_user.elxukhxvuycnftqwaghg:DeliveryOS_Api_User_Secure_123@aws-1-eu-central-1.pooler.supabase.com:6543/postgres',
  'Operational pool (6543)'
);

// Test 2: Session pool with operational user (testing if user works on port 5432)
testConnection(
  'postgresql://deliveryos_api_user.elxukhxvuycnftqwaghg:DeliveryOS_Api_User_Secure_123@aws-1-eu-central-1.pooler.supabase.com:5432/postgres',
  'Operational user on port 5432'
);

// Test 3: Actual session pool string (should fail)
testConnection(
  'postgresql://postgres.elxukhxvuycnftqwaghg:7V%23KxApMx8Z5B5.@aws-1-eu-central-1.pooler.supabase.com:5432/postgres',
  'Session pool (actual credentials)'
);