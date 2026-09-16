# Live checks

Run against the DEPLOYED service, not against a local build. Everything here
exists because a local suite passed while the live one was broken:

- the courier app showed "you are offline" for every courier while every
  endpoint answered 200;
- `/api/order/:id` was public on Cloudflare for weeks;
- a delivered order came back with `courier_id: null`, so the courier's wallet
  and history were empty while the assignment row said otherwise.

None of those are visible from a response code. Each check asks what a ROLE
SEES, which is the only question that matters.

```sh
python3 tools/live-checks/life.py    # the order, watched from all three surfaces
python3 tools/live-checks/life2.py   # the shelf, a refusal, a closed venue
python3 tools/live-checks/we2e.py    # promo codes and the money folds
bash    tools/live-checks/port.sh    # every route the consoles call
```

They are STATEFUL on purpose: they run against the real deployment with real
data. So every assertion is a delta or a shape, never an absolute — an absolute
passes once and then measures how many times the file has been run.
