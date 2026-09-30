/**
 * 设计风格卡注册表（nexu-io/open-design · design-systems/ 全量 152 张）。
 * 卡片本体在 assets/design-*.ts（scripts/import-design-systems.mjs 生成），
 * 中文名对照见 designStyleZh.ts。单独成文件：catalog/index.ts 顶 oxlint max-lines。
 */
import type { AssetManifest } from "./types.js";
import { agenticDesignAsset } from "./assets/design-agentic.js";
import { airbnbDesignAsset } from "./assets/design-airbnb.js";
import { airtableDesignAsset } from "./assets/design-airtable.js";
import { antDesignAsset } from "./assets/design-ant.js";
import { appleDesignAsset } from "./assets/design-apple.js";
import { applicationDesignAsset } from "./assets/design-application.js";
import { arcDesignAsset } from "./assets/design-arc.js";
import { artisticDesignAsset } from "./assets/design-artistic.js";
import { atelierZeroDesignAsset } from "./assets/design-atelier-zero.js";
import { bentoDesignAsset } from "./assets/design-bento.js";
import { binanceDesignAsset } from "./assets/design-binance.js";
import { bmwMDesignAsset } from "./assets/design-bmw-m.js";
import { bmwDesignAsset } from "./assets/design-bmw.js";
import { boldDesignAsset } from "./assets/design-bold.js";
import { brutalismDesignAsset } from "./assets/design-brutalism.js";
import { bugattiDesignAsset } from "./assets/design-bugatti.js";
import { cafeDesignAsset } from "./assets/design-cafe.js";
import { calDesignAsset } from "./assets/design-cal.js";
import { canvaDesignAsset } from "./assets/design-canva.js";
import { ciscoDesignAsset } from "./assets/design-cisco.js";
import { claudeDesignAsset } from "./assets/design-claude.js";
import { clayDesignAsset } from "./assets/design-clay.js";
import { claymorphismDesignAsset } from "./assets/design-claymorphism.js";
import { cleanDesignAsset } from "./assets/design-clean.js";
import { clickhouseDesignAsset } from "./assets/design-clickhouse.js";
import { cloudflareKumoDesignAsset } from "./assets/design-cloudflare-kumo.js";
import { cohereDesignAsset } from "./assets/design-cohere.js";
import { coinbaseDesignAsset } from "./assets/design-coinbase.js";
import { colorfulDesignAsset } from "./assets/design-colorful.js";
import { composioDesignAsset } from "./assets/design-composio.js";
import { contemporaryDesignAsset } from "./assets/design-contemporary.js";
import { corporateDesignAsset } from "./assets/design-corporate.js";
import { cosmicDesignAsset } from "./assets/design-cosmic.js";
import { creativeDesignAsset } from "./assets/design-creative.js";
import { cursorDesignAsset } from "./assets/design-cursor.js";
import { dashboardDesignAsset } from "./assets/design-dashboard.js";
import { defaultDesignAsset } from "./assets/design-default.js";
import { discordDesignAsset } from "./assets/design-discord.js";
import { ditheredDesignAsset } from "./assets/design-dithered.js";
import { doodleDesignAsset } from "./assets/design-doodle.js";
import { dramaticDesignAsset } from "./assets/design-dramatic.js";
import { duolingoDesignAsset } from "./assets/design-duolingo.js";
import { editorialDesignAsset } from "./assets/design-editorial.js";
import { elegantDesignAsset } from "./assets/design-elegant.js";
import { elevenlabsDesignAsset } from "./assets/design-elevenlabs.js";
import { energeticDesignAsset } from "./assets/design-energetic.js";
import { enterpriseDesignAsset } from "./assets/design-enterprise.js";
import { expoDesignAsset } from "./assets/design-expo.js";
import { expressiveDesignAsset } from "./assets/design-expressive.js";
import { fantasyDesignAsset } from "./assets/design-fantasy.js";
import { ferrariDesignAsset } from "./assets/design-ferrari.js";
import { figmaDesignAsset } from "./assets/design-figma.js";
import { flatDesignAsset } from "./assets/design-flat.js";
import { framerDesignAsset } from "./assets/design-framer.js";
import { friendlyDesignAsset } from "./assets/design-friendly.js";
import { futuristicDesignAsset } from "./assets/design-futuristic.js";
import { githubDesignAsset } from "./assets/design-github.js";
import { glassmorphismDesignAsset } from "./assets/design-glassmorphism.js";
import { gradientDesignAsset } from "./assets/design-gradient.js";
import { hashicorpDesignAsset } from "./assets/design-hashicorp.js";
import { hudDesignAsset } from "./assets/design-hud.js";
import { huggingfaceDesignAsset } from "./assets/design-huggingface.js";
import { ibmDesignAsset } from "./assets/design-ibm.js";
import { intercomDesignAsset } from "./assets/design-intercom.js";
import { kamiDesignAsset } from "./assets/design-kami.js";
import { krakenDesignAsset } from "./assets/design-kraken.js";
import { lamborghiniDesignAsset } from "./assets/design-lamborghini.js";
import { levelsDesignAsset } from "./assets/design-levels.js";
import { linearAppDesignAsset } from "./assets/design-linear-app.js";
import { lingoDesignAsset } from "./assets/design-lingo.js";
import { loomDesignAsset } from "./assets/design-loom.js";
import { lovableDesignAsset } from "./assets/design-lovable.js";
import { luxuryDesignAsset } from "./assets/design-luxury.js";
import { mastercardDesignAsset } from "./assets/design-mastercard.js";
import { materialDesignAsset } from "./assets/design-material.js";
import { metaDesignAsset } from "./assets/design-meta.js";
import { minimalDesignAsset } from "./assets/design-minimal.js";
import { minimaxDesignAsset } from "./assets/design-minimax.js";
import { mintlifyDesignAsset } from "./assets/design-mintlify.js";
import { miroDesignAsset } from "./assets/design-miro.js";
import { missionControlDesignAsset } from "./assets/design-mission-control.js";
import { mistralAiDesignAsset } from "./assets/design-mistral-ai.js";
import { modernDesignAsset } from "./assets/design-modern.js";
import { mongodbDesignAsset } from "./assets/design-mongodb.js";
import { monoDesignAsset } from "./assets/design-mono.js";
import { neobrutalismDesignAsset } from "./assets/design-neobrutalism.js";
import { neonDesignAsset } from "./assets/design-neon.js";
import { neumorphismDesignAsset } from "./assets/design-neumorphism.js";
import { nikeDesignAsset } from "./assets/design-nike.js";
import { notionDesignAsset } from "./assets/design-notion.js";
import { nvidiaDesignAsset } from "./assets/design-nvidia.js";
import { ollamaDesignAsset } from "./assets/design-ollama.js";
import { openaiDesignAsset } from "./assets/design-openai.js";
import { opencodeAiDesignAsset } from "./assets/design-opencode-ai.js";
import { pacmanDesignAsset } from "./assets/design-pacman.js";
import { paperDesignAsset } from "./assets/design-paper.js";
import { perplexityDesignAsset } from "./assets/design-perplexity.js";
import { perspectiveDesignAsset } from "./assets/design-perspective.js";
import { pinterestDesignAsset } from "./assets/design-pinterest.js";
import { playstationDesignAsset } from "./assets/design-playstation.js";
import { posthogDesignAsset } from "./assets/design-posthog.js";
import { premiumDesignAsset } from "./assets/design-premium.js";
import { professionalDesignAsset } from "./assets/design-professional.js";
import { publicationDesignAsset } from "./assets/design-publication.js";
import { raycastDesignAsset } from "./assets/design-raycast.js";
import { refinedDesignAsset } from "./assets/design-refined.js";
import { renaultDesignAsset } from "./assets/design-renault.js";
import { replicateDesignAsset } from "./assets/design-replicate.js";
import { resendDesignAsset } from "./assets/design-resend.js";
import { retroDesignAsset } from "./assets/design-retro.js";
import { revolutDesignAsset } from "./assets/design-revolut.js";
import { runwaymlDesignAsset } from "./assets/design-runwayml.js";
import { sanityDesignAsset } from "./assets/design-sanity.js";
import { sentryDesignAsset } from "./assets/design-sentry.js";
import { shadcnDesignAsset } from "./assets/design-shadcn.js";
import { shopifyDesignAsset } from "./assets/design-shopify.js";
import { simpleDesignAsset } from "./assets/design-simple.js";
import { skeumorphismDesignAsset } from "./assets/design-skeumorphism.js";
import { slackDesignAsset } from "./assets/design-slack.js";
import { sleekDesignAsset } from "./assets/design-sleek.js";
import { spacexDesignAsset } from "./assets/design-spacex.js";
import { spaciousDesignAsset } from "./assets/design-spacious.js";
import { spotifyDesignAsset } from "./assets/design-spotify.js";
import { starbucksDesignAsset } from "./assets/design-starbucks.js";
import { storytellingDesignAsset } from "./assets/design-storytelling.js";
import { stripeDesignAsset } from "./assets/design-stripe.js";
import { supabaseDesignAsset } from "./assets/design-supabase.js";
import { superhumanDesignAsset } from "./assets/design-superhuman.js";
import { teslaDesignAsset } from "./assets/design-tesla.js";
import { tetrisDesignAsset } from "./assets/design-tetris.js";
import { thevergeDesignAsset } from "./assets/design-theverge.js";
import { togetherAiDesignAsset } from "./assets/design-together-ai.js";
import { tomModernDesignAsset } from "./assets/design-tom-modern.js";
import { totalityFestivalDesignAsset } from "./assets/design-totality-festival.js";
import { tradingTerminalDesignAsset } from "./assets/design-trading-terminal.js";
import { uberDesignAsset } from "./assets/design-uber.js";
import { urduDesignAsset } from "./assets/design-urdu.js";
import { vercelDesignAsset } from "./assets/design-vercel.js";
import { vibrantDesignAsset } from "./assets/design-vibrant.js";
import { vintageDesignAsset } from "./assets/design-vintage.js";
import { vodafoneDesignAsset } from "./assets/design-vodafone.js";
import { voltagentDesignAsset } from "./assets/design-voltagent.js";
import { warmEditorialDesignAsset } from "./assets/design-warm-editorial.js";
import { warpDesignAsset } from "./assets/design-warp.js";
import { webexDesignAsset } from "./assets/design-webex.js";
import { webflowDesignAsset } from "./assets/design-webflow.js";
import { wechatDesignAsset } from "./assets/design-wechat.js";
import { wiredDesignAsset } from "./assets/design-wired.js";
import { wiseDesignAsset } from "./assets/design-wise.js";
import { xAiDesignAsset } from "./assets/design-x-ai.js";
import { xiaohongshuDesignAsset } from "./assets/design-xiaohongshu.js";
import { zapierDesignAsset } from "./assets/design-zapier.js";

export const DESIGN_STYLE_ASSETS: AssetManifest[] = [
  agenticDesignAsset,
  airbnbDesignAsset,
  airtableDesignAsset,
  antDesignAsset,
  appleDesignAsset,
  applicationDesignAsset,
  arcDesignAsset,
  artisticDesignAsset,
  atelierZeroDesignAsset,
  bentoDesignAsset,
  binanceDesignAsset,
  bmwMDesignAsset,
  bmwDesignAsset,
  boldDesignAsset,
  brutalismDesignAsset,
  bugattiDesignAsset,
  cafeDesignAsset,
  calDesignAsset,
  canvaDesignAsset,
  ciscoDesignAsset,
  claudeDesignAsset,
  clayDesignAsset,
  claymorphismDesignAsset,
  cleanDesignAsset,
  clickhouseDesignAsset,
  cloudflareKumoDesignAsset,
  cohereDesignAsset,
  coinbaseDesignAsset,
  colorfulDesignAsset,
  composioDesignAsset,
  contemporaryDesignAsset,
  corporateDesignAsset,
  cosmicDesignAsset,
  creativeDesignAsset,
  cursorDesignAsset,
  dashboardDesignAsset,
  defaultDesignAsset,
  discordDesignAsset,
  ditheredDesignAsset,
  doodleDesignAsset,
  dramaticDesignAsset,
  duolingoDesignAsset,
  editorialDesignAsset,
  elegantDesignAsset,
  elevenlabsDesignAsset,
  energeticDesignAsset,
  enterpriseDesignAsset,
  expoDesignAsset,
  expressiveDesignAsset,
  fantasyDesignAsset,
  ferrariDesignAsset,
  figmaDesignAsset,
  flatDesignAsset,
  framerDesignAsset,
  friendlyDesignAsset,
  futuristicDesignAsset,
  githubDesignAsset,
  glassmorphismDesignAsset,
  gradientDesignAsset,
  hashicorpDesignAsset,
  hudDesignAsset,
  huggingfaceDesignAsset,
  ibmDesignAsset,
  intercomDesignAsset,
  kamiDesignAsset,
  krakenDesignAsset,
  lamborghiniDesignAsset,
  levelsDesignAsset,
  linearAppDesignAsset,
  lingoDesignAsset,
  loomDesignAsset,
  lovableDesignAsset,
  luxuryDesignAsset,
  mastercardDesignAsset,
  materialDesignAsset,
  metaDesignAsset,
  minimalDesignAsset,
  minimaxDesignAsset,
  mintlifyDesignAsset,
  miroDesignAsset,
  missionControlDesignAsset,
  mistralAiDesignAsset,
  modernDesignAsset,
  mongodbDesignAsset,
  monoDesignAsset,
  neobrutalismDesignAsset,
  neonDesignAsset,
  neumorphismDesignAsset,
  nikeDesignAsset,
  notionDesignAsset,
  nvidiaDesignAsset,
  ollamaDesignAsset,
  openaiDesignAsset,
  opencodeAiDesignAsset,
  pacmanDesignAsset,
  paperDesignAsset,
  perplexityDesignAsset,
  perspectiveDesignAsset,
  pinterestDesignAsset,
  playstationDesignAsset,
  posthogDesignAsset,
  premiumDesignAsset,
  professionalDesignAsset,
  publicationDesignAsset,
  raycastDesignAsset,
  refinedDesignAsset,
  renaultDesignAsset,
  replicateDesignAsset,
  resendDesignAsset,
  retroDesignAsset,
  revolutDesignAsset,
  runwaymlDesignAsset,
  sanityDesignAsset,
  sentryDesignAsset,
  shadcnDesignAsset,
  shopifyDesignAsset,
  simpleDesignAsset,
  skeumorphismDesignAsset,
  slackDesignAsset,
  sleekDesignAsset,
  spacexDesignAsset,
  spaciousDesignAsset,
  spotifyDesignAsset,
  starbucksDesignAsset,
  storytellingDesignAsset,
  stripeDesignAsset,
  supabaseDesignAsset,
  superhumanDesignAsset,
  teslaDesignAsset,
  tetrisDesignAsset,
  thevergeDesignAsset,
  togetherAiDesignAsset,
  tomModernDesignAsset,
  totalityFestivalDesignAsset,
  tradingTerminalDesignAsset,
  uberDesignAsset,
  urduDesignAsset,
  vercelDesignAsset,
  vibrantDesignAsset,
  vintageDesignAsset,
  vodafoneDesignAsset,
  voltagentDesignAsset,
  warmEditorialDesignAsset,
  warpDesignAsset,
  webexDesignAsset,
  webflowDesignAsset,
  wechatDesignAsset,
  wiredDesignAsset,
  wiseDesignAsset,
  xAiDesignAsset,
  xiaohongshuDesignAsset,
  zapierDesignAsset,
];
