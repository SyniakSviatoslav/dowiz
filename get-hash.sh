#!/bin/sh
npm install argon2 > /dev/null 2>&1
node -e "require('argon2').hash('empty123456').then(console.log)"
