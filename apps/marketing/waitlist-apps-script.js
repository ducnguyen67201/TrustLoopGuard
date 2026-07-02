// TrustLoop waitlist webhook — paste into script.google.com
// Deploy > New deployment > Web app > Execute as: Me > Who has access: Anyone
// Then put the deployment URL into WAITLIST_WEBHOOK_URL, and set the same
// WAITLIST_WEBHOOK_SECRET in Apps Script Properties and the site environment.
function doPost(e) {
  var secret = PropertiesService.getScriptProperties().getProperty('WAITLIST_WEBHOOK_SECRET');
  var data;
  try {
    data = JSON.parse((e && e.postData && e.postData.contents) || '{}');
  } catch (err) {
    return ContentService.createTextOutput('bad request');
  }

  if (!secret || data.secret !== secret) return ContentService.createTextOutput('unauthorized');
  if (typeof data.email !== 'string' || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(data.email)) {
    return ContentService.createTextOutput('bad request');
  }

  var me = Session.getEffectiveUser().getEmail();
  var cache = CacheService.getScriptCache();
  try {
    if (!cache.get('waitlist-admin-notified')) {
      MailApp.sendEmail(me, 'TrustLoop waitlist signup', JSON.stringify(data, null, 2));
      cache.put('waitlist-admin-notified', '1', 600);
    }
    MailApp.sendEmail(
      data.email,
      "You're on the TrustLoop list",
      'One email when something ships. That was it — thanks!\n\n— TrustLoopGuard · gettrustloop.app',
    );
  } catch (err) {
    console.error('waitlist mail failed', err);
    return ContentService.createTextOutput('mail failed');
  }
  return ContentService.createTextOutput('ok');
}
