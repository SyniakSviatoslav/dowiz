Subject: Request to purge orphaned (unreferenced) git objects from <REPO>

Hi GitHub Support,

Per our security hygiene review, two rotated RSA private-key blobs were committed to this
repository in the past and later removed from all reachable history (no current branch, tag,
or ref references them). They persist only as unreferenced ("dangling") objects in the
repository's object database and remain fetchable by SHA until garbage-collected.

We have already removed them from our local clones (git gc --prune=now → 0 unreachable blobs)
and confirmed the current reachable history contains no live secrets.

Please run object expiry / `git gc --aggressive` on the server side for this repository so
the following object SHAs are no longer retrievable:

  - 478ee4459bed085d58977feb7916dcf72180e318   (RSA PRIVATE KEY, rotated, not live)
  - fa8cda34e6fde18565015e6299a24b4c274118a0   (RSA PRIVATE KEY, rotated, not live)

These keys were rotated long ago and are not valid for any current environment. This request
is solely to eliminate residual exposure of key *material* from the object store.

Thank you,
<OPERATOR>
