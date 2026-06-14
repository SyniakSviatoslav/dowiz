import * as argon2 from 'argon2';

async function main() {
  const hash = await argon2.hash('empty123456');
  console.log('HASH=', hash);
}

main().catch(console.error);
