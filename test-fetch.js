const fetch = require('node-fetch');

const BASE_URL = 'https://dowiz.fly.dev';

console.log(`Testing with BASE_URL: ${BASE_URL}`);

fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' })
  .then(res => {
    console.log(`Response status: ${res.status}`);
    return res.json();
  })
  .then(body => {
    console.log('Successfully got response:', !!body.access_token);
  })
  .catch(err => {
    console.error('Error:', err);
  });