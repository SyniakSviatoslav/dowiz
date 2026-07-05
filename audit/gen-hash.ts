import * as argon2 from 'argon2'; const hash = await argon2.hash('test123456'); console.log(hash);
