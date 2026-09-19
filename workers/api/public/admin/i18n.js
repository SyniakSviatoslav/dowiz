// The console's words, in the three languages its owners read.
//
// THE LANGUAGE SWITCH CHANGES TEXT, NOT SCREENS, the same way the storefront
// does it: every piece of static copy carries a `data-t` key and is rewritten
// in place. The console used to be written in Ukrainian only; a venue in
// Durrës is run by Albanians and its owner may well be neither.
//
// sq is the default: the venue is in Albania.

import { safeGet, safeSet } from '/store/storage.js';

export const T = {
  sq: {
    // screen hints and tile captions, 2026-09-19 design pass
    loginLine:'Paneli i lokalit tuaj, në xhep.', close:'Mbyll',
    moreHint:'Marketingu, numrat, klientët dhe çdo cilësim i lokalit. Prekni një pllakë.', menuHint:'Prekni një kategori për ta hapur, një pjatë për ta ndryshuar ose hequr nga shitja.', stockScreenHint:'Sasitë ndryshojnë vetëm me lëvizje: erdhi, u hodh, u numërua.', couriersHint:'Kush është në turn tani, kush mund të marrë porosi. Ftoni me kod.',
    promosSub:'kode zbritjeje', postsSub:'drafte për kanalet', socialSub:'Telegram, Instagram', analyticsSub:'7 dhe 30 ditë', customersSub:'të maskuar', venueSub:'emri, telefoni, adresa', hoursSub:'orari javor', deliveryTermsSub:'tarifa, minimumi', paymentsSub:'kartë, kripto', notificationsSub:'Telegram, WhatsApp', channelsSub:'nga vjen porosia', mcpSub:'agjentët', cloudSub:'kopje çdo natë', brandingSub:'ngjyra, vula', featuresSub:'ndez / fik', apiKeysSub:'për partnerët', activationSub:'gati për punë?', healthSub:'imazhet, kopja', integrationsSub:'provo çdo lidhje', assistantSub:'pyet për lokalin', inboxSub:'WhatsApp, Instagram',
    integrations:'Integrimet', integrationsHint:'Çdo lidhje e jashtme në një vend: futeni çelësat te fleta e saj dhe provojeni këtu. Prova nuk i dërgon asgjë klientit.', check:'Provo', checkAll:'Provo të gjitha', configure:'Konfiguro', lastDelivery:'Dorëzimi i fundit', noDelivery:'ende asnjë dorëzim', probeWritten:'objekt prove u shkrua', botName:'boti', followers:'ndjekës', models:'modele', webhook:'Webhook (Meta)', ai:'Asistenti AI',
    ck_no_token:'tokeni i botit s’është vendosur', ck_no_phone:'mungon tokeni ose ID e numrit', ck_no_account:'mungon tokeni ose ID e llogarisë', ck_no_verify:'fjala e verifikimit s’është vendosur', ck_no_bucket:'mungon endpoint, bucket, çelësi ose sekreti', ck_no_stripe:'çelësat e Stripe mungojnë në Worker', ck_ai_off:'asistenti është fikur', ck_handshake:'shtrëngimi i duarve u refuzua', ck_no_endpoint:'endpoint-i s’është vendosur',
    // hub-declared features and activation checks, in the reader's language
    'feat_feature.tips':'Bakshish për korrierin', 'feat_feature.tips_h':'Heq zgjedhjen e bakshishit në arkë. Bakshishet e lëna nuk humbin.',
    'feat_feature.promo':'Kodet e zbritjes', 'feat_feature.promo_h':'Fsheh fushën e kodit në arkë. Kodet mbeten, por s’ka ku të futen.',
    'feat_feature.feedback':'Vlerësim pas porosisë', 'feat_feature.feedback_h':'Heq pyetjen «si ishte?» nga faqja e porosisë.',
    'feat_feature.allergen_filter':'Filtri i alergjenëve', 'feat_feature.allergen_filter_h':'Fsheh filtrin në meny; deklaratat mbeten te pjatat.',
    'feat_feature.ar':'Pjata në tavolinë (AR)', 'feat_feature.ar_h':'Heq butonin e pamjes në madhësi reale.',
    'feat_feature.sea':'Sfondi i gjallë', 'feat_feature.sea_h':'Fik sfondin e animuar; ia vlen në telefona të vjetër.',
    'feat_feature.voice':'Komandat me zë', 'feat_feature.voice_h':'Heq mikrofonin nga panelet.',
    'feat_ai.enabled':'Asistenti AI', 'feat_ai.enabled_h':'Sa është fikur, asgjë nuk dërgohet askund.',
    'feat_social.enabled':'Draftet e postimeve', 'feat_social.enabled_h':'dowiz propozon postime për ndryshime të vërteta në meny.',
    surface_storefront:'Vitrina', surface_staff:'Panelet', changed:'ndryshuar',
    req_menu:'Menyja', req_notifications:'Njoftimet', req_fulfilment:'Dorëzimi', fact_sellableDishes:'pjata në shitje', fact_telegramChats:'Telegram', fact_hasVenuePhone:'telefoni i lokalit', fact_deliveryConfigured:'dërgesa', fact_pickupEnabled:'marrja vetë', yes:'po', no:'jo',
    // restored from the old console, 2026-09-19
    exportCsv:'Eksporto CSV', outOfStock:'S’ka në magazinë', noneOnShift:'Asnjë korrier në turn', status:'Gjendja', onShelf:'në raft', stranded:'Rezerva të varura', strandedHint:'Porosi që s’u mbyll, por mban përbërës. Numëroni raftin për t’i çliruar.',
    spoiled:'u prish', dropped:'ra', unsold:'s’u shit', inviteExpired:'kodi ka skaduar', inviteWaiting:'pret kodin', deliveries30:'Dorëzime, 30 ditë', inFlight:'Në rrugë tani', cashHeld:'Para në dorë sot',
    deactivate:'Çaktivizo korrierin', deactivateHint:'Nuk do të mund të hyjë më dhe çdo sesion i tij mbyllet.', closeVenueHint:'Porositë e reja nuk do të pranohen derisa ta hapni përsëri.',
    sizeCm:'Diametri i pjatës, cm', sizeCmHint:'Ndez pamjen «në tavolinë» te vitrina (3–120).', importHint:'CSV me kolonat category, name, price, description… Së pari provë: shihni çfarë ndryshon, pastaj zbatoni.', retireMissing:'Hiq nga shitja çfarë s’është në skedar', retireHint:'Pjatat që mungojnë në CSV kalojnë në stop-listë.', dryRun:'Provë', applyImport:'Zbato', warnings:'paralajmërime', notInFile:'jo në skedar', retired:'hequr nga shitja',
    rejected:'Refuzuar', portions:'porcione', sort_spent:'Sipas shumës', sort_orders:'Sipas numrit', sort_recent:'Të fundit', reveal:'Zbulo', revealHint:'Emri dhe telefoni tregohen një herë, për një arsye të shkruar; çdo zbulim regjistrohet.', revealReason:'Arsyeja e zbulimit', revealLog:'Regjistri i zbulimeve', lastOrders:'Porositë e fundit', lastSeen:'Parë së fundi',
    promo_active:'aktiv', promo_inactive:'ndalur', promo_scheduled:'planifikuar', promo_expired:'skaduar', promo_exhausted:'mbaruar',
    img_log:'Ditari i porosive', img_catalog:'Katalogu', img_settings:'Cilësimet', img_posts:'Postimet', img_stock:'Magazina', grows:'rritet vetë', verdict_ok:'Gjithçka në rregull', verdict_watch:'Nën vëzhgim', verdict_compact:'Duhet ndërhyrje', revokeHint:'Çelësi ndalon menjëherë; agjentët që e përdorin do të refuzohen.',
    assistant:'Asistenti', ask:'Pyet', askHint:'Një pyetje për të dhënat e gjalla të lokalit: porositë, pjatat, rezervat.', localModel:'U përgjigj modeli në këtë server', cloudModel:'Model në re · të dhënat e klientëve nuk u dërguan', aiSettings:'Modeli', aiEnabled:'Asistenti ndezur', aiEnabledHint:'Asgjë nuk dërgohet askund derisa të jetë ndezur.', aiEndpoint:'Endpoint (OpenAI-compatible /v1)', aiModel:'Modeli', aiToken:'Tokeni',
    // integrations 2026-09-19
    whatsappToken:'Tokeni i WhatsApp', whatsappPhoneId:'ID e numrit', whatsappTo:'Numri që njoftohet', whatsappHint:'Nga Meta Business: token i përhershëm dhe ID e numrit të biznesit. Numri që njoftohet: pa +, p.sh. 355691234567.', whatsappOn:'WhatsApp lidhur',
    verifyToken:'Fjala e verifikimit', verifyHint:'Çdo frazë; e njëjta vendoset te forma e webhook-ut në Meta.', appSecret:'Sekreti i aplikacionit (i nevojshëm për mesazhet)', webhookUrl:'URL e webhook-ut', webhookHint:'Ngjiteni te Meta → WhatsApp / Instagram → Webhooks, me fushat messages.',
    instagramToken:'Tokeni i Instagram', instagramUserId:'ID e llogarisë', instagramHint:'Llogari profesionale; token afatgjatë me instagram_content_publish dhe instagram_manage_messages. Postet me foto të pjatës botohen bashkë me Telegram-in.', instagramOn:'Instagram lidhur',
    inbox:'Mesazhet', inboxHint:'Klientët që shkruajnë në WhatsApp ose Instagram; përgjigjuni këtu.', noMessages:'Ende asnjë mesazh', reply:'Përgjigju', writeReply:'Shkruani përgjigjen…', unread:'pa lexuar', fromThem:'klienti', fromYou:'ju',
    cloud:'Ruajtja në re', cloudHint:'Çdo depo S3 (R2, AWS, Backblaze, MinIO). Kopja e plotë e lokalit shkon aty çdo natë dhe kur shtypni butonin.', endpoint:'Endpoint', region:'Rajoni', bucket:'Bucket', accessKey:'Access key', secretKey:'Secret key', prefix:'Dosja', pushNow:'Dërgo tani', lastCopy:'Kopja e fundit', neverPushed:'ende asnjë kopje', nightly:'çdo natë', pushed:'Kopja u dërgua',
    mcp:'Agjentët (MCP)', mcpHint:'Çdo agjent që flet MCP (Claude, Cursor…) lidhet me këtë URL me një çelës API si Bearer dhe merr veglat: porositë, menynë, magazinën, mesazhet.', tools:'vegla', telegramOn:'Telegram lidhur',
    street:'Rruga', house:'Nr.', apartment:'Ap.', entrance:'Hyrja', floor:'Kati', hoursAgo:'orë më parë', daysAgo:'ditë më parë',
    // added 2026-09-19: promos, live eta, address parts, stock, notifications
    cash:'Para në dorë', discount:'Zbritje', maxUses:'Përdorime maks.', promo:'Kodi', until:'Deri më', etaMin:'min', etaRange:'Koha e mbërritjes', onMap:'Në hartë', privateHouse:'Shtëpi private', reserved:'rezervuar',
    tgToken:'Tokeni i botit', tgTokenHint:'Krijoni një bot te @BotFather dhe ngjisni tokenin. Boti ju shkruan për çdo porosi të re.', tgChatHint:'ID e bisedës: shkruajini botit një herë, pastaj shtypni provën.', tokenSet:'tokeni është ruajtur', testOk:'Mesazhi mbërriti',
    // shell
    console:'Paneli i pronarit', signIn:'Hyni', signingIn:'Duke hyrë…', email:'Email', password:'Fjalëkalimi', signOut:'Dilni',
    language:'Gjuha', retry:'Provo përsëri', loading:'Po ngarkohet…', save:'Ruaj', saved:'U ruajt', cancel:'Anulo', done:'Në rregull',
    back:'Prapa', add:'Shto', remove:'Hiq', edit:'Ndrysho', search:'Kërko', close:'Mbyll', copy:'Kopjo', copied:'U kopjua',
    on:'Ndezur', off:'Fikur', today:'Sot', week:'7 ditë', month:'30 ditë', more:'Më shumë', none:'Asnjë', all:'Të gjitha',
    // tabs
    tabOrders:'Porositë', tabMenu:'Menyja', tabStock:'Magazina', tabCouriers:'Korrierët', tabMore:'Tjetër',
    // venue state
    open:'Hapur', closed:'Mbyllur', busy:'I zënë', paused:'Dërgesat ndalur', setState:'Gjendja e lokalit',
    // dashboard
    todayOrders:'Porosi sot', pending:'Presin', active:'Në punë', revenue:'Xhiro', scheduled:'Me orar',
    // orders
    live:'Në punë', history:'Historia', noOrders:'Ende asnjë porosi', noLive:'Asgjë në punë tani', findOrder:'Numër, emër, telefon, pjatë…',
    newOrder:'Porosi e re', order:'Porosia', items:'Artikuj', customer:'Klienti', address:'Adresa', pickup:'Marrje vetë', delivery:'Dërgesë',
    note:'Shënim', payment:'Pagesa', total:'Totali', tip:'Bakshish', when:'Kur', asap:'Sa më shpejt', courier:'Korrieri', assign:'Cakto',
    accept:'Prano', startCooking:'Fillo gatimin', markReady:'Gati', handToCourier:'Jepja korrierit', delivered:'U dorëzua', reject:'Refuzo',
    cancelOrder:'Anulo porosinë', reason:'Arsyeja', print:'Kopjo tekstin', call:'Telefono', minutesAgo:'min më parë', justNow:'tani',
    st:{PENDING:'E re',CONFIRMED:'Pranuar',PREPARING:'Po gatuhet',READY:'Gati',IN_DELIVERY:'Në rrugë',DELIVERED:'Dorëzuar',
        REJECTED:'Refuzuar',CANCELLED:'Anuluar',SCHEDULED:'Planifikuar',PICKED_UP:'Marrë'},
    pay:{cash:'Para në dorë',card:'Kartë',apple_pay:'Apple Pay',google_pay:'Google Pay',crypto:'Kripto'},
    // menu
    categories:'Kategoritë', dishes:'Pjata', onSale:'Në shitje', stopList:'Stop-lista', price:'Çmimi', photo:'Fotografia', noPhoto:'Pa fotografi',
    uploadPhoto:'Ngarko foto', removePhoto:'Hiq foton', ingredients:'Përbërësit', nutrition:'Vlerat ushqyese', kcal:'kcal', protein:'Proteina', fat:'Yndyrë', carbs:'Karbohidrate',
    weight:'Pesha, g', cookingMin:'Gatimi, min', tags:'Etiketat', translations:'Përkthimet', name:'Emri', description:'Përshkrimi',
    unavailableNote:'Pse s’ka', putOnSale:'Vëre në shitje', takeOff:'Hiqe nga shitja', importMenu:'Importo menynë', sold:'shitur', undeclared:'pa deklaruar',
    // stock
    supplies:'Furnizimet', ingredient:'Përbërësi', level:'Sasia', unit:'Njësia', low:'Pak', out:'Mbaroi', received:'U mor', wasted:'U hodh', counted:'U numërua',
    move:'Lëvizje', addSupply:'Shto furnizim', minLevel:'Minimumi', noStock:'Magazina s’është modeluar ende', stockHint:'Shtoni përbërësit që ndiqni; porosia refuzohet vetë kur mbaron diçka.',
    // couriers
    invite:'Fto korrier', inviteHint:'Kodi vlen 7 ditë; korrieri e shkruan në aplikacionin e vet.', phone:'Telefoni', inviteCode:'Kodi', onShift:'Në turn', offShift:'Jashtë turnit',
    activeC:'Aktiv', deactivate:'Çaktivizo', activate:'Aktivizo', noCouriers:'Ende asnjë korrier', deliveries:'Dorëzime', lastSeen:'Parë së fundi', uninvite:'Anulo ftesën',
    // more
    marketing:'Marketing', promos:'Kodet e zbritjes', posts:'Postimet', analytics:'Analitika', customers:'Klientët', settings:'Cilësimet', health:'Gjendja e sistemit',
    notifications:'Njoftimet', channels:'Kanalet e porosive', integrations:'Integrimet', branding:'Marka', hours:'Orari', deliveryTerms:'Dërgesa', payments:'Pagesat',
    features:'Funksionet', apiKeys:'Çelësat API', backup:'Rezervë', activation:'Aktivizimi', venue:'Lokali',
    // notifications
    telegram:'Telegram', whatsapp:'WhatsApp', tgHow:'Lidhni botin: shkruani /start botit nga telefoni i lokalit — porositë e reja vijnë aty.', tgChannel:'Kanali i postimeve',
    tgNotSet:'Boti i Telegram-it nuk është konfiguruar në këtë hub', waNotYet:'WhatsApp Business nuk është lidhur ende — kërkon një llogari WhatsApp Business API.',
    ownerChat:'Chat-i i pronarit', linked:'I lidhur', notLinked:'Jo i lidhur', testMessage:'Dërgo mesazh prove',
    // channels
    chStore:'Dyqani online', chPhone:'Me telefon', chTelegramBot:'Bot Telegram', chApi:'API / partnerë', chAggregators:'Agregatorë (Wolt, Glovo…)', comingSoon:'Së shpejti',
    apiHint:'Një çelës API lejon një sistem tjetër (kasë, agregator) të lexojë menynë dhe të dërgojë porosi.', newKey:'Çelës i ri', revoke:'Revoko', keyOnce:'Kopjojeni tani — nuk shfaqet më.',
    // social
    social:'Rrjetet sociale', autopost:'Autopostim', autopostHint:'Hubi propozon postime nga fakte të vërteta (pjatë e re, më e porositura, u hap). Asgjë s’publikohet pa ju.',
    draft:'Draft', approve:'Publiko', rejectPost:'Hiqe', drafts:'Draftet', published:'Publikuar', failed:'Dështoi', noPosts:'Ende asnjë draft', makeDraft:'Propozo postim',
    instagram:'Instagram', facebook:'Facebook', tiktok:'TikTok', socialNotYet:'nuk është lidhur ende',
    // settings
    venueName:'Emri', venuePhone:'Telefoni', venueAddress:'Adresa', deliveryFee:'Tarifa e dërgesës', freeOver:'Falas nga', minOrder:'Porosia minimale',
    pickupOn:'Marrje vetë', stripe:'Stripe (kartë, Apple/Google Pay)', stripeNotSet:'Çelësat e Stripe s’janë vendosur në hub', cryptoWallets:'Portofolat kripto', network:'Rrjeti', symbol:'Monedha', walletAddress:'Adresa',
    seal:'Vula', motif:'Motivi', warmTone:'Ngjyra e ngrohtë', sageTone:'Ngjyra e gjelbër', leaf:'Gjethe', wave:'Valë', noneMotif:'Pa motiv',
    primary:'Ngjyra kryesore', paper:'Letra', typePair:'Shkronjat', radius:'Rrumbullakimi', preview:'Parapamja',
    day:{0:'E hënë',1:'E martë',2:'E mërkurë',3:'E enjte',4:'E premte',5:'E shtunë',6:'E diel'}, closedDay:'Mbyllur',
    // analytics
    orders7:'Porosi', revenue7:'Xhiro', avgCheck:'Çeku mesatar', topDishes:'Më të porositurat', byHour:'Sipas orës', byDay:'Sipas ditës',
    // health
    hubOk:'Hubi në rregull', latency:'Vonesa', images:'Imazhet', lastBackup:'Rezerva e fundit', download:'Shkarko',
    // errors
    loadFail:'Nuk u ngarkua', sessionOver:'Sesioni mbaroi', required:'E detyrueshme',
  },
  en: {
    // screen hints and tile captions, 2026-09-19 design pass
    loginLine:'Your venue\'s console, in your pocket.', close:'Close',
    moreHint:'Marketing, numbers, customers and every venue setting. Tap a tile.', menuHint:'Tap a category to open it, a dish to change it or take it off sale.', stockScreenHint:'Quantities change only by movements: received, wasted, counted.', couriersHint:'Who is on shift now, who may take orders. Invite by code.',
    promosSub:'discount codes', postsSub:'drafts for channels', socialSub:'Telegram, Instagram', analyticsSub:'7 and 30 days', customersSub:'masked', venueSub:'name, phone, address', hoursSub:'weekly schedule', deliveryTermsSub:'fee, minimum', paymentsSub:'card, crypto', notificationsSub:'Telegram, WhatsApp', channelsSub:'where orders come from', mcpSub:'agents', cloudSub:'a copy every night', brandingSub:'colours, seal', featuresSub:'on / off', apiKeysSub:'for partners', activationSub:'ready to work?', healthSub:'images, backup', integrationsSub:'prove every connection', assistantSub:'ask about the venue', inboxSub:'WhatsApp, Instagram',
    integrations:'Integrations', integrationsHint:'Every outside connection in one place: enter the keys in its sheet, prove it here. A check sends nothing to a customer.', check:'Check', checkAll:'Check all', configure:'Set up', lastDelivery:'Last delivery', noDelivery:'no delivery yet', probeWritten:'probe object written', botName:'bot', followers:'followers', models:'models', webhook:'Webhook (Meta)', ai:'AI assistant',
    ck_no_token:'no bot token is set', ck_no_phone:'token or phone number id missing', ck_no_account:'token or account id missing', ck_no_verify:'no verify token is set', ck_no_bucket:'endpoint, bucket, key or secret missing', ck_no_stripe:'Stripe keys are missing on the Worker', ck_ai_off:'the assistant is off', ck_handshake:'the handshake was refused', ck_no_endpoint:'no endpoint is set',
    // hub-declared features and activation checks, in the reader's language
    'feat_feature.tips':'Courier tips', 'feat_feature.tips_h':'Removes the tip choice at checkout. Tips already left stay.',
    'feat_feature.promo':'Promo codes', 'feat_feature.promo_h':'Hides the code field at checkout. Codes remain, but nowhere to type them.',
    'feat_feature.feedback':'Feedback after the order', 'feat_feature.feedback_h':'Removes the “how was it?” field from the order page.',
    'feat_feature.allergen_filter':'Allergen filter', 'feat_feature.allergen_filter_h':'Hides the filter in the menu; declarations stay on dishes.',
    'feat_feature.ar':'Dish on the table (AR)', 'feat_feature.ar_h':'Removes the real-size view button.',
    'feat_feature.sea':'Living background', 'feat_feature.sea_h':'Turns the animated background off; worth it on old phones.',
    'feat_feature.voice':'Voice commands', 'feat_feature.voice_h':'Removes the microphone from the panels.',
    'feat_ai.enabled':'AI assistant', 'feat_ai.enabled_h':'While off, nothing is sent anywhere.',
    'feat_social.enabled':'Post drafts', 'feat_social.enabled_h':'dowiz drafts posts about real changes to the menu.',
    surface_storefront:'Storefront', surface_staff:'Panels', changed:'changed',
    req_menu:'Menu', req_notifications:'Notifications', req_fulfilment:'Fulfilment', fact_sellableDishes:'dishes on sale', fact_telegramChats:'Telegram', fact_hasVenuePhone:'venue phone', fact_deliveryConfigured:'delivery', fact_pickupEnabled:'pickup', yes:'yes', no:'no',
    // restored from the old console, 2026-09-19
    exportCsv:'Export CSV', outOfStock:'Out of stock', noneOnShift:'No courier on shift', status:'Status', onShelf:'on shelf', stranded:'Stranded holds', strandedHint:'Orders that never closed but still hold ingredients. A stocktake releases them.',
    spoiled:'spoiled', dropped:'dropped', unsold:'unsold', inviteExpired:'code expired', inviteWaiting:'waiting for code', deliveries30:'Deliveries, 30 days', inFlight:'On the road now', cashHeld:'Cash in hand today',
    deactivate:'Deactivate courier', deactivateHint:'They will no longer be able to sign in and every session of theirs ends.', closeVenueHint:'No new orders will come in until you open again.',
    sizeCm:'Plate diameter, cm', sizeCmHint:'Turns on the “on the table” view in the storefront (3–120).', importHint:'A CSV with columns category, name, price, description… Dry run first: see what changes, then apply.', retireMissing:'Take off sale what is not in the file', retireHint:'Dishes missing from the CSV go to the stop list.', dryRun:'Dry run', applyImport:'Apply', warnings:'warnings', notInFile:'not in file', retired:'taken off sale',
    rejected:'Rejected', portions:'portions', sort_spent:'By spend', sort_orders:'By orders', sort_recent:'Recent', reveal:'Reveal', revealHint:'Name and phone are shown once, for a written reason; every reveal is logged.', revealReason:'Reason for revealing', revealLog:'Reveal log', lastOrders:'Last orders', lastSeen:'Last seen',
    promo_active:'active', promo_inactive:'paused', promo_scheduled:'scheduled', promo_expired:'expired', promo_exhausted:'used up',
    img_log:'Order log', img_catalog:'Catalogue', img_settings:'Settings', img_posts:'Posts', img_stock:'Stock', grows:'grows by itself', verdict_ok:'All well', verdict_watch:'Under watch', verdict_compact:'Needs attention', revokeHint:'The key stops at once; any agent using it will be refused.',
    assistant:'Assistant', ask:'Ask', askHint:'A question about the venue\'s own live data: orders, dishes, stock.', localModel:'Answered by the model on this server', cloudModel:'Cloud model · customer data was not sent', aiSettings:'Model', aiEnabled:'Assistant on', aiEnabledHint:'Nothing is sent anywhere until this is on.', aiEndpoint:'Endpoint (OpenAI-compatible /v1)', aiModel:'Model', aiToken:'Token',
    // integrations 2026-09-19
    whatsappToken:'WhatsApp token', whatsappPhoneId:'Phone number id', whatsappTo:'Number to notify', whatsappHint:'From Meta Business: a permanent token and the business number\'s id. Number to notify without +, e.g. 355691234567.', whatsappOn:'WhatsApp connected',
    verifyToken:'Verify token', verifyHint:'Any phrase; paste the same one into the webhook form in Meta.', appSecret:'App secret (required for the inbox)', webhookUrl:'Webhook URL', webhookHint:'Paste into Meta → WhatsApp / Instagram → Webhooks, subscribed to messages.',
    instagramToken:'Instagram token', instagramUserId:'Account id', instagramHint:'Professional account; a long-lived token with instagram_content_publish and instagram_manage_messages. Posts with a dish photo go out beside Telegram.', instagramOn:'Instagram connected',
    inbox:'Messages', inboxHint:'Customers who write on WhatsApp or Instagram; answer them here.', noMessages:'No messages yet', reply:'Reply', writeReply:'Write a reply…', unread:'unread', fromThem:'customer', fromYou:'you',
    cloud:'Cloud storage', cloudHint:'Any S3 store (R2, AWS, Backblaze, MinIO). A full copy of the venue lands there every night and when you press the button.', endpoint:'Endpoint', region:'Region', bucket:'Bucket', accessKey:'Access key', secretKey:'Secret key', prefix:'Folder', pushNow:'Push now', lastCopy:'Last copy', neverPushed:'no copy yet', nightly:'nightly', pushed:'Copy pushed',
    mcp:'Agents (MCP)', mcpHint:'Any MCP-speaking agent (Claude, Cursor…) connects to this URL with an API key as Bearer and gets the tools: orders, menu, stock, messages.', tools:'tools', telegramOn:'Telegram connected',
    street:'Street', house:'No.', apartment:'Apt', entrance:'Entrance', floor:'Floor', hoursAgo:'h ago', daysAgo:'d ago',
    // added 2026-09-19: promos, live eta, address parts, stock, notifications
    cash:'Cash', discount:'Discount', maxUses:'Max uses', promo:'Code', until:'Until', etaMin:'min', etaRange:'Arrives in', onMap:'On the map', privateHouse:'Private house', reserved:'held',
    tgToken:'Bot token', tgTokenHint:'Make a bot with @BotFather and paste its token. It messages you about every new order.', tgChatHint:'Chat id: write to the bot once, then press test.', tokenSet:'token is saved', testOk:'The message arrived',
    console:'Owner console', signIn:'Sign in', signingIn:'Signing in…', email:'Email', password:'Password', signOut:'Sign out',
    language:'Language', retry:'Try again', loading:'Loading…', save:'Save', saved:'Saved', cancel:'Cancel', done:'Done',
    back:'Back', add:'Add', remove:'Remove', edit:'Edit', search:'Search', close:'Close', copy:'Copy', copied:'Copied',
    on:'On', off:'Off', today:'Today', week:'7 days', month:'30 days', more:'More', none:'None', all:'All',
    tabOrders:'Orders', tabMenu:'Menu', tabStock:'Stock', tabCouriers:'Couriers', tabMore:'More',
    open:'Open', closed:'Closed', busy:'Busy', paused:'Delivery paused', setState:'Venue state',
    todayOrders:'Orders today', pending:'Waiting', active:'In progress', revenue:'Revenue', scheduled:'Scheduled',
    live:'Live', history:'History', noOrders:'No orders yet', noLive:'Nothing in progress', findOrder:'Number, name, phone, dish…',
    newOrder:'New order', order:'Order', items:'Items', customer:'Customer', address:'Address', pickup:'Pickup', delivery:'Delivery',
    note:'Note', payment:'Payment', total:'Total', tip:'Tip', when:'When', asap:'As soon as possible', courier:'Courier', assign:'Assign',
    accept:'Accept', startCooking:'Start cooking', markReady:'Ready', handToCourier:'Hand to courier', delivered:'Delivered', reject:'Reject',
    cancelOrder:'Cancel order', reason:'Reason', print:'Copy as text', call:'Call', minutesAgo:'min ago', justNow:'just now',
    st:{PENDING:'New',CONFIRMED:'Accepted',PREPARING:'Cooking',READY:'Ready',IN_DELIVERY:'On the way',DELIVERED:'Delivered',
        REJECTED:'Rejected',CANCELLED:'Cancelled',SCHEDULED:'Scheduled',PICKED_UP:'Picked up'},
    pay:{cash:'Cash',card:'Card',apple_pay:'Apple Pay',google_pay:'Google Pay',crypto:'Crypto'},
    categories:'Categories', dishes:'Dishes', onSale:'On sale', stopList:'Stop list', price:'Price', photo:'Photo', noPhoto:'No photo',
    uploadPhoto:'Upload photo', removePhoto:'Remove photo', ingredients:'Ingredients', nutrition:'Nutrition', kcal:'kcal', protein:'Protein', fat:'Fat', carbs:'Carbs',
    weight:'Weight, g', cookingMin:'Cooking, min', tags:'Tags', translations:'Translations', name:'Name', description:'Description',
    unavailableNote:'Why unavailable', putOnSale:'Put on sale', takeOff:'Take off sale', importMenu:'Import menu', sold:'sold', undeclared:'undeclared',
    supplies:'Supplies', ingredient:'Ingredient', level:'Level', unit:'Unit', low:'Low', out:'Out', received:'Received', wasted:'Wasted', counted:'Counted',
    move:'Movement', addSupply:'Add supply', minLevel:'Minimum', noStock:'Stock is not modelled yet', stockHint:'Add the ingredients you track; an order is refused by itself when something runs out.',
    invite:'Invite courier', inviteHint:'The code is valid for 7 days; the courier types it in their app.', phone:'Phone', inviteCode:'Code', onShift:'On shift', offShift:'Off shift',
    activeC:'Active', deactivate:'Deactivate', activate:'Activate', noCouriers:'No couriers yet', deliveries:'Deliveries', lastSeen:'Last seen', uninvite:'Cancel invite',
    marketing:'Marketing', promos:'Promo codes', posts:'Posts', analytics:'Analytics', customers:'Customers', settings:'Settings', health:'System health',
    notifications:'Notifications', channels:'Order channels', integrations:'Integrations', branding:'Brand', hours:'Hours', deliveryTerms:'Delivery', payments:'Payments',
    features:'Features', apiKeys:'API keys', backup:'Backup', activation:'Activation', venue:'Venue',
    telegram:'Telegram', whatsapp:'WhatsApp', tgHow:'Link the bot: send /start to the bot from the venue phone — new orders arrive there.', tgChannel:'Posting channel',
    tgNotSet:'The Telegram bot is not configured on this hub', waNotYet:'WhatsApp Business is not linked yet — it needs a WhatsApp Business API account.',
    ownerChat:'Owner chat', linked:'Linked', notLinked:'Not linked', testMessage:'Send a test message',
    chStore:'Online storefront', chPhone:'By phone', chTelegramBot:'Telegram bot', chApi:'API / partners', chAggregators:'Aggregators (Wolt, Glovo…)', comingSoon:'Coming soon',
    apiHint:'An API key lets another system (a till, an aggregator) read the menu and send orders.', newKey:'New key', revoke:'Revoke', keyOnce:'Copy it now — it is not shown again.',
    social:'Social media', autopost:'Autoposting', autopostHint:'The hub drafts posts from true facts (a new dish, the most ordered, reopened). Nothing publishes without you.',
    draft:'Draft', approve:'Publish', rejectPost:'Discard', drafts:'Drafts', published:'Published', failed:'Failed', noPosts:'No drafts yet', makeDraft:'Draft a post',
    instagram:'Instagram', facebook:'Facebook', tiktok:'TikTok', socialNotYet:'not linked yet',
    venueName:'Name', venuePhone:'Phone', venueAddress:'Address', deliveryFee:'Delivery fee', freeOver:'Free from', minOrder:'Minimum order',
    pickupOn:'Pickup', stripe:'Stripe (card, Apple/Google Pay)', stripeNotSet:'Stripe keys are not set on the hub', cryptoWallets:'Crypto wallets', network:'Network', symbol:'Coin', walletAddress:'Address',
    seal:'Seal', motif:'Motif', warmTone:'Warm tone', sageTone:'Leaf tone', leaf:'Leaf', wave:'Wave', noneMotif:'No motif',
    primary:'Primary colour', paper:'Paper', typePair:'Type', radius:'Corner radius', preview:'Preview',
    day:{0:'Monday',1:'Tuesday',2:'Wednesday',3:'Thursday',4:'Friday',5:'Saturday',6:'Sunday'}, closedDay:'Closed',
    orders7:'Orders', revenue7:'Revenue', avgCheck:'Average check', topDishes:'Most ordered', byHour:'By hour', byDay:'By day',
    hubOk:'Hub is fine', latency:'Latency', images:'Images', lastBackup:'Last backup', download:'Download',
    loadFail:'Could not load', sessionOver:'Session expired', required:'Required',
  },
  uk: {
    // screen hints and tile captions, 2026-09-19 design pass
    loginLine:'Консоль вашого закладу в кишені.', close:'Закрити',
    moreHint:'Маркетинг, цифри, клієнти й усі налаштування закладу. Торкніться плитки.', menuHint:'Торкніться категорії, щоб розгорнути, страви, щоб змінити або зняти з продажу.', stockScreenHint:'Кількості змінюються лише рухами: прийшло, списано, порахували.', couriersHint:'Хто зараз на зміні, хто може брати замовлення. Запрошуйте кодом.',
    promosSub:'коди знижок', postsSub:'чернетки для каналів', socialSub:'Telegram, Instagram', analyticsSub:'7 і 30 днів', customersSub:'замасковані', venueSub:'назва, телефон, адреса', hoursSub:'тижневий графік', deliveryTermsSub:'вартість, мінімум', paymentsSub:'картка, крипто', notificationsSub:'Telegram, WhatsApp', channelsSub:'звідки замовлення', mcpSub:'агенти', cloudSub:'копія щоночі', brandingSub:'кольори, печатка', featuresSub:'увімк / вимк', apiKeysSub:'для партнерів', activationSub:'готово до роботи?', healthSub:'образи, копія', integrationsSub:'перевірити кожне', assistantSub:'спитати про заклад', inboxSub:'WhatsApp, Instagram',
    integrations:'Інтеграції', integrationsHint:'Усі зовнішні підключення в одному місці: введіть ключі у відповідному аркуші й перевірте тут. Перевірка нічого не надсилає клієнтам.', check:'Перевірити', checkAll:'Перевірити все', configure:'Налаштувати', lastDelivery:'Остання доставка', noDelivery:'доставок ще не було', probeWritten:'тестовий об’єкт записано', botName:'бот', followers:'підписників', models:'моделей', webhook:'Вебхук (Meta)', ai:'AI-асистент',
    ck_no_token:'токен бота не задано', ck_no_phone:'бракує токена або ID номера', ck_no_account:'бракує токена або ID акаунта', ck_no_verify:'слово перевірки не задано', ck_no_bucket:'бракує endpoint, bucket, ключа або секрету', ck_no_stripe:'ключів Stripe немає у Worker', ck_ai_off:'асистент вимкнений', ck_handshake:'рукостискання відхилено', ck_no_endpoint:'endpoint не задано',
    // hub-declared features and activation checks, in the reader's language
    'feat_feature.tips':'Чайові кур’єру', 'feat_feature.tips_h':'Прибирає вибір чайових на касі. Уже залишені чайові нікуди не зникають.',
    'feat_feature.promo':'Промокоди', 'feat_feature.promo_h':'Ховає поле коду на касі. Створені коди лишаються, але ввести їх ніде.',
    'feat_feature.feedback':'Відгук після замовлення', 'feat_feature.feedback_h':'Прибирає поле «як вам?» зі сторінки замовлення.',
    'feat_feature.allergen_filter':'Фільтр за алергенами', 'feat_feature.allergen_filter_h':'Ховає фільтр у меню; заяви лишаються на стравах.',
    'feat_feature.ar':'Страва на столі (AR)', 'feat_feature.ar_h':'Прибирає кнопку перегляду страви в реальному розмірі.',
    'feat_feature.sea':'Рухливе тло', 'feat_feature.sea_h':'Вимикає анімоване тло; варте того на старих телефонах.',
    'feat_feature.voice':'Голосові команди', 'feat_feature.voice_h':'Прибирає мікрофон з панелей.',
    'feat_ai.enabled':'AI-помічник', 'feat_ai.enabled_h':'Поки вимкнено, нікуди нічого не надсилається.',
    'feat_social.enabled':'Чернетки постів', 'feat_social.enabled_h':'dowiz пропонує пости про справжні зміни в меню.',
    surface_storefront:'Вітрина', surface_staff:'Панелі', changed:'змінено',
    req_menu:'Меню', req_notifications:'Сповіщення', req_fulfilment:'Доставка', fact_sellableDishes:'страв у продажу', fact_telegramChats:'Telegram', fact_hasVenuePhone:'телефон закладу', fact_deliveryConfigured:'доставка', fact_pickupEnabled:'самовивіз', yes:'так', no:'ні',
    // restored from the old console, 2026-09-19
    exportCsv:'Експорт CSV', outOfStock:'Немає в наявності', noneOnShift:'Немає кур’єрів на зміні', status:'Статус', onShelf:'на полиці', stranded:'Зависли резерви', strandedHint:'Замовлення, що не закрилися, але тримають інгредієнти. Інвентаризація їх звільняє.',
    spoiled:'зіпсувалось', dropped:'впало', unsold:'не продано', inviteExpired:'код прострочено', inviteWaiting:'чекає на код', deliveries30:'Доставок за 30 днів', inFlight:'Зараз у дорозі', cashHeld:'Готівка на руках сьогодні',
    deactivate:'Деактивувати кур’єра', deactivateHint:'Він більше не зможе увійти, усі його сесії завершаться.', closeVenueHint:'Нові замовлення не надходитимуть, доки ви не відкриєтесь знову.',
    sizeCm:'Діаметр тарілки, см', sizeCmHint:'Вмикає перегляд «на столі» у вітрині (3–120).', importHint:'CSV з колонками category, name, price, description… Спершу пробний запуск: побачите зміни, потім застосуйте.', retireMissing:'Зняти з продажу те, чого немає у файлі', retireHint:'Страви, відсутні у CSV, підуть у стоп-лист.', dryRun:'Пробний запуск', applyImport:'Застосувати', warnings:'попереджень', notInFile:'нема у файлі', retired:'знято з продажу',
    rejected:'Відхилено', portions:'порцій', sort_spent:'За сумою', sort_orders:'За кількістю', sort_recent:'Нещодавні', reveal:'Розкрити', revealHint:'Ім’я і телефон показуються один раз, із записаною причиною; кожне розкриття потрапляє в журнал.', revealReason:'Причина розкриття', revealLog:'Журнал розкриттів', lastOrders:'Останні замовлення', lastSeen:'Востаннє',
    promo_active:'активний', promo_inactive:'на паузі', promo_scheduled:'заплановано', promo_expired:'минув', promo_exhausted:'вичерпано',
    img_log:'Журнал замовлень', img_catalog:'Каталог', img_settings:'Налаштування', img_posts:'Пости', img_stock:'Склад', grows:'росте сам', verdict_ok:'Все добре', verdict_watch:'Під наглядом', verdict_compact:'Потрібне втручання', revokeHint:'Ключ зупиниться одразу; агентам, що ним користуються, буде відмовлено.',
    assistant:'Асистент', ask:'Запитати', askHint:'Питання про живі дані закладу: замовлення, страви, резерви.', localModel:'Відповіла модель на цьому сервері', cloudModel:'Хмарна модель · дані клієнтів не надсилались', aiSettings:'Модель', aiEnabled:'Асистент увімкнено', aiEnabledHint:'Нічого нікуди не надсилається, поки це вимкнено.', aiEndpoint:'Endpoint (OpenAI-сумісний /v1)', aiModel:'Модель', aiToken:'Токен',
    // integrations 2026-09-19
    whatsappToken:'Токен WhatsApp', whatsappPhoneId:'ID номера', whatsappTo:'Номер для сповіщень', whatsappHint:'З Meta Business: постійний токен і ID бізнес-номера. Номер для сповіщень без +, напр. 355691234567.', whatsappOn:'WhatsApp підключено',
    verifyToken:'Слово перевірки', verifyHint:'Будь-яка фраза; ту саму вставте у форму вебхука в Meta.', appSecret:'Секрет застосунку (потрібен для повідомлень)', webhookUrl:'URL вебхука', webhookHint:'Вставте в Meta → WhatsApp / Instagram → Webhooks, з підпискою на messages.',
    instagramToken:'Токен Instagram', instagramUserId:'ID акаунта', instagramHint:'Професійний акаунт; довгий токен з instagram_content_publish та instagram_manage_messages. Пости з фото страви виходять разом із Telegram.', instagramOn:'Instagram підключено',
    inbox:'Повідомлення', inboxHint:'Клієнти, що пишуть у WhatsApp чи Instagram; відповідайте тут.', noMessages:'Повідомлень ще немає', reply:'Відповісти', writeReply:'Напишіть відповідь…', unread:'непрочитані', fromThem:'клієнт', fromYou:'ви',
    cloud:'Хмарне сховище', cloudHint:'Будь-яке S3-сховище (R2, AWS, Backblaze, MinIO). Повна копія закладу лягає туди щоночі та за кнопкою.', endpoint:'Endpoint', region:'Регіон', bucket:'Bucket', accessKey:'Access key', secretKey:'Secret key', prefix:'Тека', pushNow:'Надіслати зараз', lastCopy:'Остання копія', neverPushed:'копій ще немає', nightly:'щоночі', pushed:'Копію надіслано',
    mcp:'Агенти (MCP)', mcpHint:'Будь-який агент із MCP (Claude, Cursor…) підключається до цієї URL з API-ключем як Bearer і отримує інструменти: замовлення, меню, склад, повідомлення.', tools:'інструментів', telegramOn:'Telegram підключено',
    street:'Вулиця', house:'Буд.', apartment:'Кв.', entrance:'Під’їзд', floor:'Поверх', hoursAgo:'год тому', daysAgo:'дн тому',
    // added 2026-09-19: promos, live eta, address parts, stock, notifications
    cash:'Готівка', discount:'Знижка', maxUses:'Макс. використань', promo:'Код', until:'До', etaMin:'хв', etaRange:'Прибуде за', onMap:'На мапі', privateHouse:'Приватний будинок', reserved:'зарезервовано',
    tgToken:'Токен бота', tgTokenHint:'Створіть бота у @BotFather і вставте токен. Бот писатиме вам про кожне нове замовлення.', tgChatHint:'ID чату: напишіть боту один раз, потім натисніть перевірку.', tokenSet:'токен збережено', testOk:'Повідомлення дійшло',
    console:'Панель власника', signIn:'Увійти', signingIn:'Входимо…', email:'Email', password:'Пароль', signOut:'Вийти',
    language:'Мова', retry:'Спробувати ще', loading:'Завантажуємо…', save:'Зберегти', saved:'Збережено', cancel:'Скасувати', done:'Готово',
    back:'Назад', add:'Додати', remove:'Прибрати', edit:'Змінити', search:'Пошук', close:'Закрити', copy:'Копіювати', copied:'Скопійовано',
    on:'Увімкнено', off:'Вимкнено', today:'Сьогодні', week:'7 днів', month:'30 днів', more:'Ще', none:'Немає', all:'Усі',
    tabOrders:'Замовлення', tabMenu:'Меню', tabStock:'Склад', tabCouriers:'Кур’єри', tabMore:'Ще',
    open:'Відчинено', closed:'Зачинено', busy:'Зайнято', paused:'Доставку призупинено', setState:'Стан закладу',
    todayOrders:'Замовлень сьогодні', pending:'Чекають', active:'У роботі', revenue:'Виручка', scheduled:'На час',
    live:'У роботі', history:'Історія', noOrders:'Замовлень ще немає', noLive:'Зараз нічого в роботі', findOrder:'Номер, ім’я, телефон, страва…',
    newOrder:'Нове замовлення', order:'Замовлення', items:'Позиції', customer:'Клієнт', address:'Адреса', pickup:'Самовивіз', delivery:'Доставка',
    note:'Коментар', payment:'Оплата', total:'Разом', tip:'Чайові', when:'Коли', asap:'Якнайшвидше', courier:'Кур’єр', assign:'Призначити',
    accept:'Прийняти', startCooking:'Готувати', markReady:'Готово', handToCourier:'Віддати кур’єру', delivered:'Доставлено', reject:'Відхилити',
    cancelOrder:'Скасувати замовлення', reason:'Причина', print:'Скопіювати текстом', call:'Зателефонувати', minutesAgo:'хв тому', justNow:'щойно',
    st:{PENDING:'Нове',CONFIRMED:'Прийнято',PREPARING:'Готується',READY:'Готове',IN_DELIVERY:'У дорозі',DELIVERED:'Доставлено',
        REJECTED:'Відхилено',CANCELLED:'Скасовано',SCHEDULED:'Заплановано',PICKED_UP:'Забрано'},
    pay:{cash:'Готівка',card:'Картка',apple_pay:'Apple Pay',google_pay:'Google Pay',crypto:'Крипто'},
    categories:'Категорії', dishes:'Страви', onSale:'У продажу', stopList:'Стоп-лист', price:'Ціна', photo:'Фото', noPhoto:'Без фото',
    uploadPhoto:'Завантажити фото', removePhoto:'Прибрати фото', ingredients:'Інгредієнти', nutrition:'Харчова цінність', kcal:'ккал', protein:'Білки', fat:'Жири', carbs:'Вуглеводи',
    weight:'Вага, г', cookingMin:'Готування, хв', tags:'Теги', translations:'Переклади', name:'Назва', description:'Опис',
    unavailableNote:'Чому немає', putOnSale:'У продаж', takeOff:'Зняти з продажу', importMenu:'Імпортувати меню', sold:'продано', undeclared:'не заявлено',
    supplies:'Постачання', ingredient:'Інгредієнт', level:'Залишок', unit:'Одиниця', low:'Мало', out:'Закінчилось', received:'Прийнято', wasted:'Списано', counted:'Перераховано',
    move:'Рух', addSupply:'Додати позицію', minLevel:'Мінімум', noStock:'Склад ще не змодельовано', stockHint:'Додайте інгредієнти, які відстежуєте; замовлення саме відхилиться, коли щось закінчиться.',
    invite:'Запросити кур’єра', inviteHint:'Код дійсний 7 днів; кур’єр вводить його у своєму застосунку.', phone:'Телефон', inviteCode:'Код', onShift:'На зміні', offShift:'Не на зміні',
    activeC:'Активний', deactivate:'Деактивувати', activate:'Активувати', noCouriers:'Кур’єрів ще немає', deliveries:'Доставок', lastSeen:'Востаннє', uninvite:'Скасувати запрошення',
    marketing:'Маркетинг', promos:'Промокоди', posts:'Публікації', analytics:'Аналітика', customers:'Клієнти', settings:'Налаштування', health:'Стан системи',
    notifications:'Сповіщення', channels:'Канали замовлень', integrations:'Інтеграції', branding:'Бренд', hours:'Години роботи', deliveryTerms:'Доставка', payments:'Оплата',
    features:'Функції', apiKeys:'API-ключі', backup:'Резервна копія', activation:'Активація', venue:'Заклад',
    telegram:'Telegram', whatsapp:'WhatsApp', tgHow:'Підключіть бота: напишіть йому /start з телефону закладу — нові замовлення приходитимуть туди.', tgChannel:'Канал для публікацій',
    tgNotSet:'Telegram-бот на цьому хабі не налаштований', waNotYet:'WhatsApp Business ще не підключено — потрібен акаунт WhatsApp Business API.',
    ownerChat:'Чат власника', linked:'Підключено', notLinked:'Не підключено', testMessage:'Надіслати тестове повідомлення',
    chStore:'Онлайн-вітрина', chPhone:'По телефону', chTelegramBot:'Telegram-бот', chApi:'API / партнери', chAggregators:'Агрегатори (Wolt, Glovo…)', comingSoon:'Незабаром',
    apiHint:'API-ключ дає іншій системі (касі, агрегатору) читати меню й надсилати замовлення.', newKey:'Новий ключ', revoke:'Відкликати', keyOnce:'Скопіюйте зараз — більше не покажемо.',
    social:'Соцмережі', autopost:'Автопостинг', autopostHint:'Хаб пропонує пости з правдивих фактів (нова страва, найпопулярніша, відкриття). Нічого не публікується без вас.',
    draft:'Чернетка', approve:'Опублікувати', rejectPost:'Відхилити', drafts:'Чернетки', published:'Опубліковано', failed:'Не вдалося', noPosts:'Чернеток ще немає', makeDraft:'Запропонувати пост',
    instagram:'Instagram', facebook:'Facebook', tiktok:'TikTok', socialNotYet:'ще не підключено',
    venueName:'Назва', venuePhone:'Телефон', venueAddress:'Адреса', deliveryFee:'Вартість доставки', freeOver:'Безкоштовно від', minOrder:'Мінімальне замовлення',
    pickupOn:'Самовивіз', stripe:'Stripe (картка, Apple/Google Pay)', stripeNotSet:'Ключі Stripe на хабі не задано', cryptoWallets:'Криптогаманці', network:'Мережа', symbol:'Монета', walletAddress:'Адреса',
    seal:'Печатка', motif:'Мотив', warmTone:'Теплий тон', sageTone:'Тон листя', leaf:'Листя', wave:'Хвиля', noneMotif:'Без мотиву',
    primary:'Основний колір', paper:'Папір', typePair:'Шрифт', radius:'Заокруглення', preview:'Попередній перегляд',
    day:{0:'Понеділок',1:'Вівторок',2:'Середа',3:'Четвер',4:'Пʼятниця',5:'Субота',6:'Неділя'}, closedDay:'Зачинено',
    orders7:'Замовлень', revenue7:'Виручка', avgCheck:'Середній чек', topDishes:'Найпопулярніші', byHour:'За годинами', byDay:'За днями',
    hubOk:'Хаб у нормі', latency:'Затримка', images:'Образи', lastBackup:'Остання копія', download:'Завантажити',
    loadFail:'Не завантажилось', sessionOver:'Сесія завершилась', required:'Обов’язково',
  },
};

export const LANGS = ['sq', 'en', 'uk'];
export let lang = LANGS.includes(safeGet('dw_admin_lang')) ? safeGet('dw_admin_lang') : 'sq';
export const t = k => (T[lang] && T[lang][k]) ?? T.en[k] ?? k;
export const st = s => (t('st')[s]) || s;
export const payName = p => (t('pay')[p]) || p || '';
export const intlLocale = () => lang === 'uk' ? 'uk' : lang === 'en' ? 'en' : 'sq';

/// Ukrainian needs three forms; Albanian and English two.
export function plural(n, key){
  const forms = t(key);
  if (!Array.isArray(forms)) return String(forms);
  if (lang !== 'uk') return n === 1 ? forms[0] : forms[forms.length - 1];
  const m10 = n % 10, m100 = n % 100;
  if (m10 === 1 && m100 !== 11) return forms[0];
  if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return forms[1];
  return forms[2];
}

export function setLang(code){
  if (!LANGS.includes(code) || code === lang) return false;
  lang = code;
  safeSet('dw_admin_lang', lang);
  document.documentElement.lang = lang;
  retranslate(document);
  return true;
}

/// Rewrite text under `root`: `data-t="key"`, `data-t-attr="placeholder:key"`,
/// `data-t-st="STATUS"`. Nothing else moves.
export function retranslate(root = document){
  for (const el of root.querySelectorAll('[data-t]')) {
    const v = t(el.dataset.t);
    if (typeof v === 'string' && el.textContent !== v) el.textContent = v;
  }
  for (const el of root.querySelectorAll('[data-t-attr]')) {
    for (const pair of el.dataset.tAttr.split(/\s+/)) {
      const [attr, key] = pair.split(':');
      if (attr && key) el.setAttribute(attr, t(key));
    }
  }
  for (const el of root.querySelectorAll('[data-t-st]')) el.textContent = st(el.dataset.tSt);
}
