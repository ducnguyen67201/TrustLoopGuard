// TrustLoop waitlist webhook — paste into script.google.com
// Deploy > New deployment > Web app > Execute as: Me > Who has access: Anyone
// Then put the deployment URL into WAITLIST_WEBHOOK_URL.
function doPost(e) {
  const data = JSON.parse(e.postData.contents);
  const me = Session.getEffectiveUser().getEmail();
  MailApp.sendEmail(me, 'TrustLoop waitlist signup: ' + data.email, JSON.stringify(data, null, 2));
  MailApp.sendEmail(
    data.email,
    "You're on the TrustLoop list",
    'One email when something ships. That was it — thanks!\n\n— TrustLoopGuard · gettrustloop.app',
  );
  return ContentService.createTextOutput('ok');
}
