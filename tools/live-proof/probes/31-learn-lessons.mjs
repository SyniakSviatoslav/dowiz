// Row 31: the lessons the console plays come out of R2 (bucket dowiz-learn,
// binding LEARN) through the Worker. The manifest names each cut's size and
// sha256 as tools/learn/publish.sh wrote them; the bytes served must be those
// bytes, so the R2 object is checked without an R2 credential.
import crypto from 'node:crypto';
export default async function ({ lib, check, must, note }) {
  const m = await lib.own(`/api/learn/manifest?location_id=${lib.LOC}`);
  must(m.status === 200, `manifest ${m.status} ${m.text.slice(0, 100)}`);
  check('response_schema', m.body);
  const lessons = Object.entries(m.body.lessons || {});
  must(lessons.length, 'the manifest lists no lessons');
  const [id, lesson] = lessons[0];
  const [lang, cut] = Object.entries(lesson.cuts || {})[0] || [];
  must(cut?.video && cut.bytes && cut.sha256, `lesson ${id} has no cut with video, bytes and sha256`);
  const r = await fetch(`${lib.HOST}${cut.video}`, { headers: { 'user-agent': lib.UA, authorization: `Bearer ${await lib.owner()}` } });
  const buf = Buffer.from(await r.arrayBuffer());
  const type = r.headers.get('content-type') || '';
  must(r.status === 200 && /^(video|image)\//.test(type), `${cut.video}: ${r.status} ${type}`);
  must(buf.length === cut.bytes, `${cut.video}: ${buf.length} B served, manifest says ${cut.bytes}`);
  const sha = crypto.createHash('sha256').update(buf).digest('hex');
  must(sha.startsWith(cut.sha256), `${cut.video}: sha256 ${sha.slice(0, 16)} != manifest ${cut.sha256}`);
  note(`${lessons.length} lessons; ${id}/${lang} video ${buf.length} B ${type}, sha256 ${cut.sha256} == manifest`);
}
