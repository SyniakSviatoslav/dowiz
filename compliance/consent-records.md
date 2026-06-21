# Записи згоди — де і як (це ДАНІ, не документи)
> Згода доводиться РЯДКАМИ В БД з timestamp+версією, не файлами.

Наявний патерн (вже є): access_requests.consent_at + privacy_version.
Розширити той самий патерн на:
- акцепт DPA власником (owner_id, dpa_version, accepted_at)
- згода клієнта при зборі (+ privacy_version/ts; marketing_opt_in вже є)
- push-згода (customer_devices opt-in ts)

Тригер: нова точка збору згоди → додати ts+version у відповідну таблицю.
