set -u
U=https://dowiz-api.sviatoslavsyniak.workers.dev
ok=0; fail=0
T=$(curl -s -X POST $U/api/auth/login -H 'content-type: application/json' \
  -d '{"email":"ana@dubin.al","password":"dubin-owner"}'|python3 -c 'import sys,json;print(json.load(sys.stdin)["access_token"])')
chk(){ # name expected actual
  if [ "$2" = "$3" ]; then printf '  ✓ %s\n' "$1"; ok=$((ok+1))
  else printf '  ✗ %s  очікував %s, отримав %s\n' "$1" "$2" "$3"; fail=$((fail+1)); fi; }
g(){ curl -s -o /dev/null -w "%{http_code}" "$U$1" -H "authorization: Bearer $T" -H "accept: application/json"; }
p(){ curl -s -o /dev/null -w '%{http_code}' -X POST "$U$1" -H "authorization: Bearer $T" -H 'content-type: application/json' -d "$2"; }

echo "— налаштування —"
chk "GET /owner/settings" 200 "$(g /api/owner/settings)"; sleep 0.4
chk "невідомий ключ відхилено" 400 "$(p /api/owner/settings '{"key":"nope","value":"x"}')"; sleep 0.4
chk "http-ендпойнт відхилено" 400 "$(p /api/owner/settings '{"key":"ai.endpoint","value":"http://1.2.3.4/v1"}')"; sleep 0.4
chk "https прийнято" 200 "$(p /api/owner/settings '{"key":"ai.endpoint","value":"https://api.groq.com/openai/v1"}')"; sleep 0.4

echo "— пости —"
chk "GET /owner/posts" 200 "$(g /api/owner/posts)"; sleep 0.4
chk "чернетка без дозволу" 409 "$(p /api/owner/posts/draft '{}')"; sleep 0.4

echo "— помічник —"
chk "вимкнений помічник" 409 "$(p /api/owner/assist '{"question":"скільки замовлень"}')"; sleep 0.4

echo "— кур'єри —"
chk "GET /owner/couriers" 200 "$(g /api/owner/couriers)"; sleep 0.4
chk "запрошення на зайнятий номер" 409 "$(p /api/owner/couriers/invite '{"phone":"+355691112233","name":"X"}')"; sleep 0.4

echo "— ключі —"
chk "ключ без назви" 400 "$(p /api/owner/apikeys '{"label":"  "}')"; sleep 0.4
chk "GET /owner/apikeys" 200 "$(g /api/owner/apikeys)"; sleep 0.4

echo "— імпорт —"
chk "прев'ю імпорту" 200 "$(curl -s -o /dev/null -w '%{http_code}' -X POST "$U/api/owner/menu/import" -H "authorization: Bearer $T" -H 'content-type: text/csv' --data-binary 'Розділ,Назва,Ціна
Test,Проба,500')"
printf '\n%s пройшло, %s впало\n' "$ok" "$fail"
