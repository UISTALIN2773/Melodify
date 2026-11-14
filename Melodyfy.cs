using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.IO;
using System.Net;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using System.Windows.Forms;
using System.Web.Script.Serialization;

namespace Melodyfy
{
    public class Track
    {
        public string Id { get; set; }
        public string Title { get; set; }
        public string Artist { get; set; }
        public string Duration { get; set; }
        public int DurationSeconds { get; set; }
        public string ThumbnailUrl { get; set; }
        public string WebpageUrl { get; set; }
        public Image ThumbnailImage { get; set; } // Cached image
    }

    public class PlayerForm : Form
    {
        // Paleta Light Minimalista (Regla DEV01) + Ámbar Editorial
        private Color bgSidebar = Color.FromArgb(248, 249, 250); // Crema/Gris muy claro
        private Color bgMain = Color.FromArgb(255, 255, 255); // Blanco puro
        private Color bgBottom = Color.FromArgb(255, 255, 255); // Blanco
        private Color colText = Color.FromArgb(10, 10, 10); // Texto oscuro (#0a0a0a)
        private Color colSubtext = Color.FromArgb(120, 120, 120); // Gris apagado
        private Color colAccent = Color.FromArgb(217, 119, 54); // Ámbar/Naranja quemado
        private Color colHover = Color.FromArgb(240, 240, 240); // Hover claro

        // Tipografía Editorial
        private FontFamily serifFont = new FontFamily("Georgia");
        private FontFamily sansFont = new FontFamily("Segoe UI");

        // UI
        private Panel titleBar;
        private Panel sidebar;
        private Panel bottomBar;
        private Panel mainArea;
        private Panel headerPanel;
        private TextBox searchBox;
        private DataGridView trackList;
        private PictureBox pbHeaderIcon;
        private Label lblHeaderTitle;
        private Label lblHeaderSubtitle;

        // Player Controls
        private PictureBox pbAlbumArt;
        private Label lblNowPlayingTitle;
        private Label lblNowPlayingArtist;
        private Button btnPlayPause;
        private TrackBar progressSlider;
        private TrackBar volumeSlider;
        private Label lblCurrentTime;
        private Label lblTotalTime;
        private Label lblVolIcon;
        private Button btnMiniPlayer;

        // Estado
        private dynamic wmp;
        private List<Track> currentTracks = new List<Track>();
        private Track currentPlayingTrack = null;
        private Timer playbackTimer;
        private bool isMiniPlayer = false;
        private Size normalSize = new Size(1100, 750);
        private Size miniSize = new Size(400, 90);
        private int hoveredRow = -1;

        // API
        public const int WM_NCLBUTTONDOWN = 0xA1;
        public const int HT_CAPTION = 0x2;
        [DllImport("user32.dll")]
        public static extern int SendMessage(IntPtr hWnd, int Msg, int wParam, int lParam);
        [DllImport("user32.dll")]
        public static extern bool ReleaseCapture();

        public PlayerForm()
        {
            ServicePointManager.SecurityProtocol = (SecurityProtocolType)3072;
            this.DoubleBuffered = true; 
            InitializeComponent();
            InitializePlayer();
            this.Load += async (s, e) => await SearchYouTubeAsync("Música Gaming 2026");
        }

        private void InitializeComponent()
        {
            this.Size = normalSize;
            this.Text = "Melodyfy";
            this.BackColor = bgMain;
            this.StartPosition = FormStartPosition.CenterScreen;
            this.FormBorderStyle = FormBorderStyle.None;
            this.Font = new Font(sansFont, 10);

            // TITLE BAR
            titleBar = new Panel() { Height = 30, Dock = DockStyle.Top, BackColor = bgSidebar };
            titleBar.MouseDown += DragWindow_MouseDown;
            Button btnClose = new Button() { Text = "✕", ForeColor = colSubtext, FlatStyle = FlatStyle.Flat, Size = new Size(40, 30), Location = new Point(this.Width - 40, 0), Anchor = AnchorStyles.Right | AnchorStyles.Top, Cursor = Cursors.Hand };
            btnClose.FlatAppearance.BorderSize = 0; btnClose.Click += (s, e) => Application.Exit();
            btnMiniPlayer = new Button() { Text = "🕹️", ForeColor = colSubtext, FlatStyle = FlatStyle.Flat, Size = new Size(40, 30), Location = new Point(this.Width - 80, 0), Anchor = AnchorStyles.Right | AnchorStyles.Top, Cursor = Cursors.Hand };
            btnMiniPlayer.FlatAppearance.BorderSize = 0; btnMiniPlayer.Click += ToggleMiniPlayer;
            titleBar.Controls.Add(btnClose); titleBar.Controls.Add(btnMiniPlayer);
            this.Controls.Add(titleBar);

            // SIDEBAR
            sidebar = new Panel() { Width = 240, Dock = DockStyle.Left, BackColor = bgSidebar };
            Label appLogo = new Label() { Text = "🎧 Melodyfy", ForeColor = colText, Font = new Font(serifFont, 16, FontStyle.Bold), Location = new Point(20, 20), AutoSize = true };
            sidebar.Controls.Add(appLogo);
            
            Button btnHome = CreateSidebarBtn("🏠  Inicio", 80);
            Button btnSearch = CreateSidebarBtn("🔍  Buscar", 125);
            Button btnLibrary = CreateSidebarBtn("📚  Tu Biblioteca", 170);
            sidebar.Controls.Add(btnHome); sidebar.Controls.Add(btnSearch); sidebar.Controls.Add(btnLibrary);

            // BOTTOM BAR (Reproductor)
            bottomBar = new Panel() { Height = 90, Dock = DockStyle.Bottom, BackColor = bgBottom };
            bottomBar.MouseDown += DragWindow_MouseDown;
            pbAlbumArt = new PictureBox() { Location = new Point(15, 15), Size = new Size(60, 60), SizeMode = PictureBoxSizeMode.Zoom, BackColor = Color.FromArgb(240,240,240) };
            lblNowPlayingTitle = new Label() { Text = "Sin reproducir", ForeColor = colText, Font = new Font(serifFont, 12, FontStyle.Bold), Location = new Point(90, 25), AutoSize = true };
            lblNowPlayingArtist = new Label() { Text = "-", ForeColor = colSubtext, Font = new Font(sansFont, 9), Location = new Point(90, 48), AutoSize = true };
            bottomBar.Controls.Add(pbAlbumArt); bottomBar.Controls.Add(lblNowPlayingTitle); bottomBar.Controls.Add(lblNowPlayingArtist);

            Panel centerControls = new Panel() { Width = 400, Height = 90, BackColor = Color.Transparent };
            btnPlayPause = new Button() { Text = "▶", ForeColor = Color.White, BackColor = colAccent, FlatStyle = FlatStyle.Flat, Size = new Size(40, 40), Location = new Point(180, 8), Font = new Font(sansFont, 12, FontStyle.Bold), Cursor = Cursors.Hand };
            btnPlayPause.FlatAppearance.BorderSize = 0;
            // Rounded play button
            GraphicsPath pPath = new GraphicsPath(); pPath.AddEllipse(0, 0, 40, 40); btnPlayPause.Region = new Region(pPath);
            btnPlayPause.Click += BtnPlayPause_Click;

            lblCurrentTime = new Label() { Text = "0:00", ForeColor = colSubtext, Font = new Font(sansFont, 8), Location = new Point(0, 55), AutoSize = true };
            progressSlider = new TrackBar() { Location = new Point(35, 52), Width = 330, Height = 20, TickStyle = TickStyle.None, Maximum = 100 };
            progressSlider.Scroll += (s, e) => { if (wmp != null) { try { wmp.controls.currentPosition = progressSlider.Value; } catch { } } };
            lblTotalTime = new Label() { Text = "0:00", ForeColor = colSubtext, Font = new Font(sansFont, 8), Location = new Point(370, 55), AutoSize = true };

            centerControls.Controls.Add(btnPlayPause); centerControls.Controls.Add(lblCurrentTime); centerControls.Controls.Add(progressSlider); centerControls.Controls.Add(lblTotalTime);
            bottomBar.Controls.Add(centerControls);
            
            lblVolIcon = new Label() { Text = "🔊", ForeColor = colSubtext, Font = new Font(sansFont, 10), AutoSize = true };
            volumeSlider = new TrackBar() { Width = 100, Height = 20, TickStyle = TickStyle.None, Maximum = 100, Value = 100, Cursor = Cursors.Hand };
            volumeSlider.Scroll += (s, e) => { if (wmp != null) { try { wmp.settings.volume = volumeSlider.Value; File.WriteAllText("vol.txt", volumeSlider.Value.ToString()); } catch { } } };
            
            bottomBar.Controls.Add(lblVolIcon);
            bottomBar.Controls.Add(volumeSlider);
            
            bottomBar.Resize += (s, e) => { 
                centerControls.Location = new Point((bottomBar.Width - centerControls.Width) / 2, 0); 
                lblVolIcon.Location = new Point(bottomBar.Width - 140, 35);
                volumeSlider.Location = new Point(bottomBar.Width - 110, 35);
            };

            // MAIN AREA
            mainArea = new Panel() { Dock = DockStyle.Fill, BackColor = bgMain };

            // HEADER GRADIENT
            headerPanel = new Panel() { Dock = DockStyle.Top, Height = 260, BackColor = Color.Transparent };
            headerPanel.Paint += HeaderPanel_Paint;
            
            searchBox = new TextBox() { Text = "", Font = new Font(sansFont, 12), Width = 350, Location = new Point(30, 20), BackColor = Color.White, ForeColor = colText, BorderStyle = BorderStyle.FixedSingle };
            searchBox.KeyDown += SearchBox_KeyDown;
            
            pbHeaderIcon = new PictureBox() { Location = new Point(30, 70), Size = new Size(160, 160), BackColor = Color.Transparent, SizeMode = PictureBoxSizeMode.Zoom };
            lblHeaderSubtitle = new Label() { Text = "Playlist Pública", ForeColor = colText, Font = new Font(sansFont, 10), Location = new Point(210, 120), AutoSize = true, BackColor = Color.Transparent };
            lblHeaderTitle = new Label() { Text = "Buenas tardes", ForeColor = colText, Font = new Font(serifFont, 48, FontStyle.Bold), Location = new Point(200, 135), AutoSize = true, BackColor = Color.Transparent };

            headerPanel.Controls.Add(searchBox);
            headerPanel.Controls.Add(pbHeaderIcon);
            headerPanel.Controls.Add(lblHeaderSubtitle);
            headerPanel.Controls.Add(lblHeaderTitle);

            // TRACK LIST CUSTOM
            trackList = new DataGridView();
            trackList.Location = new Point(0, 260);
            trackList.Size = new Size(mainArea.Width, mainArea.Height - 260);
            trackList.Anchor = AnchorStyles.Top | AnchorStyles.Bottom | AnchorStyles.Left | AnchorStyles.Right;
            trackList.BackgroundColor = bgMain;
            trackList.BorderStyle = BorderStyle.None;
            trackList.CellBorderStyle = DataGridViewCellBorderStyle.None;
            trackList.ColumnHeadersVisible = false;
            trackList.RowHeadersVisible = false;
            trackList.AllowUserToAddRows = false;
            trackList.ReadOnly = true;
            trackList.SelectionMode = DataGridViewSelectionMode.FullRowSelect;
            trackList.RowTemplate.Height = 60;
            trackList.DefaultCellStyle.BackColor = bgMain;
            trackList.GridColor = bgMain;
            trackList.ShowCellToolTips = false;

            trackList.Columns.Add("Index", "#");
            trackList.Columns.Add("Info", "Info");
            trackList.Columns.Add("Duration", "Duración");
            trackList.Columns[0].Width = 50;
            trackList.Columns[1].AutoSizeMode = DataGridViewAutoSizeColumnMode.Fill;
            trackList.Columns[2].Width = 100;

            trackList.CellPainting += TrackList_CellPainting;
            trackList.CellDoubleClick += TrackList_CellDoubleClick;
            trackList.CellMouseClick += TrackList_CellMouseClick;
            trackList.CellMouseMove += (s, e) => { if (e.RowIndex != hoveredRow) { hoveredRow = e.RowIndex; trackList.Invalidate(); } };
            trackList.MouseLeave += (s, e) => { hoveredRow = -1; trackList.Invalidate(); };

            mainArea.Controls.Add(headerPanel);
            mainArea.Controls.Add(trackList);

            this.Controls.Add(mainArea);
            this.Controls.Add(sidebar);
            this.Controls.Add(bottomBar);

            playbackTimer = new Timer() { Interval = 1000 };
            playbackTimer.Tick += PlaybackTimer_Tick;
        }

        private async void TrackList_CellMouseClick(object sender, DataGridViewCellMouseEventArgs e)
        {
            if (e.RowIndex >= 0 && e.RowIndex < currentTracks.Count && e.ColumnIndex == 0)
            {
                await PlayTrackAsync(currentTracks[e.RowIndex]);
            }
        }

        private async void PlayNextTrack()
        {
            if (currentPlayingTrack != null)
            {
                int idx = currentTracks.FindIndex(x => x.Id == currentPlayingTrack.Id);
                if (idx >= 0)
                {
                    int nextIdx = (idx + 1) % currentTracks.Count; // Fix: Reproducción cíclica (loop continuo)
                    await PlayTrackAsync(currentTracks[nextIdx]);
                }
            }
        }

        private Button CreateSidebarBtn(string text, int y)
        {
            Button b = new Button() { Text = text, Location = new Point(20, y), Size = new Size(200, 40), FlatStyle = FlatStyle.Flat, ForeColor = colSubtext, Font = new Font(sansFont, 11, FontStyle.Bold), TextAlign = ContentAlignment.MiddleLeft, Cursor = Cursors.Hand };
            b.FlatAppearance.BorderSize = 0; b.FlatAppearance.MouseOverBackColor = Color.FromArgb(230,230,230);
            b.MouseEnter += (s, e) => b.ForeColor = colText; b.MouseLeave += (s, e) => b.ForeColor = colSubtext;
            return b;
        }

        private void HeaderPanel_Paint(object sender, PaintEventArgs e)
        {
            // Gradient background: from light gray/amber mix to pure white
            using (LinearGradientBrush brush = new LinearGradientBrush(headerPanel.ClientRectangle, Color.FromArgb(240, 230, 220), bgMain, 90F))
            {
                e.Graphics.FillRectangle(brush, headerPanel.ClientRectangle);
            }
        }

        private void TrackList_CellPainting(object sender, DataGridViewCellPaintingEventArgs e)
        {
            if (e.RowIndex >= 0 && e.ColumnIndex >= 0 && e.RowIndex < currentTracks.Count)
            {
                e.Handled = true;
                e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;

                Track t = currentTracks[e.RowIndex];
                bool isHovered = (e.RowIndex == hoveredRow);
                bool isSelected = trackList.Rows[e.RowIndex].Selected;
                bool isPlaying = (currentPlayingTrack != null && currentPlayingTrack.Id == t.Id);

                // Background
                Color bgColor = isSelected ? colHover : (isHovered ? Color.FromArgb(250,250,250) : bgMain);
                using (SolidBrush bgBrush = new SolidBrush(bgColor)) {
                    e.Graphics.FillRectangle(bgBrush, e.CellBounds);
                }

                // Columna 0: Índice o botón Play
                if (e.ColumnIndex == 0)
                {
                    Rectangle rect = e.CellBounds;
                    if (isHovered) {
                        TextRenderer.DrawText(e.Graphics, "▶", new Font(sansFont, 12), rect, colText, TextFormatFlags.VerticalCenter | TextFormatFlags.HorizontalCenter);
                    } else if (isPlaying) {
                        TextRenderer.DrawText(e.Graphics, "🎵", new Font(sansFont, 12), rect, colAccent, TextFormatFlags.VerticalCenter | TextFormatFlags.HorizontalCenter);
                    } else {
                        TextRenderer.DrawText(e.Graphics, (e.RowIndex + 1).ToString(), new Font(sansFont, 10), rect, colSubtext, TextFormatFlags.VerticalCenter | TextFormatFlags.HorizontalCenter);
                    }
                }
                // Columna 1: Título y Artista
                else if (e.ColumnIndex == 1)
                {
                    Rectangle titleRect = new Rectangle(e.CellBounds.X + 10, e.CellBounds.Y + 10, e.CellBounds.Width - 20, 22);
                    Rectangle artistRect = new Rectangle(e.CellBounds.X + 10, e.CellBounds.Y + 32, e.CellBounds.Width - 20, 20);
                    Color titleColor = isPlaying ? colAccent : colText;
                    
                    TextRenderer.DrawText(e.Graphics, t.Title, new Font(serifFont, 12, FontStyle.Bold), titleRect, titleColor, TextFormatFlags.VerticalCenter | TextFormatFlags.Left | TextFormatFlags.EndEllipsis);
                    TextRenderer.DrawText(e.Graphics, t.Artist, new Font(sansFont, 9), artistRect, colSubtext, TextFormatFlags.VerticalCenter | TextFormatFlags.Left | TextFormatFlags.EndEllipsis);
                }
                // Columna 2: Duración
                else if (e.ColumnIndex == 2)
                {
                    Rectangle rect = new Rectangle(e.CellBounds.X, e.CellBounds.Y, e.CellBounds.Width - 20, e.CellBounds.Height);
                    TextRenderer.DrawText(e.Graphics, t.Duration, new Font(sansFont, 10), rect, colSubtext, TextFormatFlags.VerticalCenter | TextFormatFlags.Right);
                }
            }
        }

        private void InitializePlayer() { try { Type wmpType = Type.GetTypeFromProgID("WMPlayer.OCX"); wmp = Activator.CreateInstance(wmpType); int vol = 100; if (File.Exists("vol.txt")) int.TryParse(File.ReadAllText("vol.txt"), out vol); wmp.settings.volume = vol; if (volumeSlider != null) volumeSlider.Value = vol; } catch { } }

        private void DragWindow_MouseDown(object sender, MouseEventArgs e) { if (e.Button == MouseButtons.Left) { ReleaseCapture(); SendMessage(Handle, WM_NCLBUTTONDOWN, HT_CAPTION, 0); } }

        private void ApplyRoundedRegion(int radius)
        {
            GraphicsPath path = new GraphicsPath();
            path.AddArc(0, 0, radius, radius, 180, 90); path.AddArc(this.Width - radius, 0, radius, radius, 270, 90);
            path.AddArc(this.Width - radius, this.Height - radius, radius, radius, 0, 90); path.AddArc(0, this.Height - radius, radius, radius, 90, 90);
            path.CloseFigure(); this.Region = new Region(path);
        }

        private void ToggleMiniPlayer(object sender, EventArgs e)
        {
            if (!isMiniPlayer) {
                isMiniPlayer = true; this.SuspendLayout();
                mainArea.Visible = false; sidebar.Visible = false; titleBar.Visible = false;
                this.Size = miniSize; this.TopMost = true; this.Opacity = 0.90; 
                this.BackColor = Color.Black; bottomBar.BackColor = Color.Black;
                btnMiniPlayer.Parent = bottomBar; btnMiniPlayer.Location = new Point(this.Width - 50, 30); btnMiniPlayer.Text = "🔙";
                this.Location = new Point((Screen.PrimaryScreen.Bounds.Width - this.Width) / 2, 20);
                ApplyRoundedRegion(40); this.ResumeLayout();
            } else {
                isMiniPlayer = false; this.SuspendLayout();
                this.Region = null; this.Size = normalSize;
                this.BackColor = bgMain; bottomBar.BackColor = bgBottom;
                mainArea.Visible = true; sidebar.Visible = true; titleBar.Visible = true;
                this.TopMost = false; this.Opacity = 1.0; 
                btnMiniPlayer.Parent = titleBar; btnMiniPlayer.Location = new Point(this.Width - 80, 0); btnMiniPlayer.Text = "🕹️";
                this.Location = new Point((Screen.PrimaryScreen.Bounds.Width - this.Width) / 2, (Screen.PrimaryScreen.Bounds.Height - this.Height) / 2);
                this.ResumeLayout();
            }
            ForceGarbageCollection();
        }

        private async void SearchBox_KeyDown(object sender, KeyEventArgs e)
        {
            if (e.KeyCode == Keys.Enter) {
                e.SuppressKeyPress = true; string query = searchBox.Text.Trim();
                if (!string.IsNullOrEmpty(query)) await SearchYouTubeAsync(query);
            }
        }

        private async Task SearchYouTubeAsync(string query)
        {
            lblHeaderTitle.Text = "Buscando...";
            trackList.Rows.Clear(); currentTracks.Clear();

            string ytDlpPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "yt-dlp.exe");
            if (!File.Exists(ytDlpPath)) return;

            try {
                string args = "--dump-json \"ytsearch20:" + query + "\"";
                if (query.Contains("http") && (query.Contains("list=") || query.Contains("playlist"))) {
                    args = "--dump-json --flat-playlist \"" + query + "\"";
                }

                ProcessStartInfo psi = new ProcessStartInfo { FileName = ytDlpPath, Arguments = args, RedirectStandardOutput = true, UseShellExecute = false, CreateNoWindow = true, StandardOutputEncoding = System.Text.Encoding.UTF8 };

                await Task.Run(() => {
                    using (Process process = Process.Start(psi)) {
                        JavaScriptSerializer serializer = new JavaScriptSerializer();
                        using (StreamReader reader = process.StandardOutput) {
                            string line;
                            while ((line = reader.ReadLine()) != null) {
                                try {
                                    var dict = serializer.Deserialize<Dictionary<string, object>>(line);
                                    if (dict != null && dict.ContainsKey("id")) {
                                        Track t = new Track();
                                        t.Id = dict.ContainsKey("id") ? dict["id"].ToString() : "";
                                        t.Title = dict.ContainsKey("title") ? dict["title"].ToString() : "Unknown";
                                        t.Artist = dict.ContainsKey("uploader") ? dict["uploader"].ToString() : (dict.ContainsKey("channel") ? dict["channel"].ToString() : "Unknown");
                                        
                                        if (dict.ContainsKey("webpage_url")) t.WebpageUrl = dict["webpage_url"].ToString();
                                        else if (dict.ContainsKey("url")) t.WebpageUrl = dict["url"].ToString();

                                        if (string.IsNullOrEmpty(t.WebpageUrl)) t.WebpageUrl = "https://www.youtube.com/watch?v=" + t.Id;

                                        if (dict.ContainsKey("thumbnails") && dict["thumbnails"] is System.Collections.ArrayList) {
                                            var thumbs = (System.Collections.ArrayList)dict["thumbnails"];
                                            if (thumbs.Count > 0) {
                                                var firstThumb = thumbs[0] as Dictionary<string, object>;
                                                if (firstThumb != null && firstThumb.ContainsKey("url")) t.ThumbnailUrl = firstThumb["url"].ToString();
                                            }
                                        }
                                        if (string.IsNullOrEmpty(t.ThumbnailUrl) && dict.ContainsKey("thumbnail")) t.ThumbnailUrl = dict["thumbnail"].ToString();
                                        
                                        if (dict.ContainsKey("duration")) {
                                            int seconds = Convert.ToInt32(dict["duration"]); t.DurationSeconds = seconds;
                                            t.Duration = (seconds / 60).ToString() + ":" + (seconds % 60).ToString("00");
                                        }

                                        currentTracks.Add(t);
                                        trackList.Invoke((MethodInvoker)delegate { trackList.Rows.Add("", "", ""); }); // Filas vacías, se dibujan manual
                                    }
                                } catch { }
                            }
                        }
                        process.WaitForExit();
                    }
                });
                lblHeaderTitle.Text = query.Contains("list=") ? "Tu Playlist" : query;
                
                // Cargar portada gigante
                if (currentTracks.Count > 0 && !string.IsNullOrEmpty(currentTracks[0].ThumbnailUrl)) {
                    Task.Run(() => {
                        try {
                            using (WebClient wc = new WebClient()) {
                                byte[] bytes = wc.DownloadData(currentTracks[0].ThumbnailUrl);
                                using (MemoryStream ms = new MemoryStream(bytes)) {
                                    Image img = Image.FromStream(ms);
                                    Image clonedImg = new Bitmap(img);
                                    pbHeaderIcon.Invoke((MethodInvoker)delegate { 
                                        if (pbHeaderIcon.Image != null) pbHeaderIcon.Image.Dispose();
                                        pbHeaderIcon.Image = clonedImg; 
                                    });
                                }
                            }
                        } catch { }
                    });
                }
            }
            catch { lblHeaderTitle.Text = "Error"; }
            ForceGarbageCollection();
        }

        private async void TrackList_CellDoubleClick(object sender, DataGridViewCellEventArgs e) { if (e.RowIndex >= 0 && e.RowIndex < currentTracks.Count) await PlayTrackAsync(currentTracks[e.RowIndex]); }

        private async Task PlayTrackAsync(Track t)
        {
            currentPlayingTrack = t;
            lblNowPlayingTitle.Text = t.Title.Length > 40 ? t.Title.Substring(0, 40) + "..." : t.Title;
            lblNowPlayingArtist.Text = t.Artist;
            lblTotalTime.Text = t.Duration;
            progressSlider.Maximum = t.DurationSeconds > 0 ? t.DurationSeconds : 100;
            progressSlider.Value = 0;
            btnPlayPause.Text = "⏳";
            pbAlbumArt.Image = null;
            trackList.Invalidate(); // Redraw para mostrar texto verde en la cancion actual

            if (!string.IsNullOrEmpty(t.ThumbnailUrl)) {
                Task.Run(() => {
                    try {
                        using (WebClient wc = new WebClient()) {
                            byte[] bytes = wc.DownloadData(t.ThumbnailUrl);
                            using (MemoryStream ms = new MemoryStream(bytes)) {
                                Image img = Image.FromStream(ms);
                                Image clonedImg = new Bitmap(img);
                                pbAlbumArt.Invoke((MethodInvoker)delegate { 
                                    if (pbAlbumArt.Image != null) pbAlbumArt.Image.Dispose();
                                    pbAlbumArt.Image = clonedImg; 
                                });
                            }
                        }
                    } catch { }
                });
            }

            string ytDlpPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "yt-dlp.exe");
            ProcessStartInfo psi = new ProcessStartInfo { FileName = ytDlpPath, Arguments = "-g -f \"bestaudio[ext=m4a]/bestaudio\" \"" + t.WebpageUrl + "\"", RedirectStandardOutput = true, UseShellExecute = false, CreateNoWindow = true };

            try {
                string streamUrl = await Task.Run(() => {
                    using (Process process = Process.Start(psi)) {
                        string url = process.StandardOutput.ReadLine(); process.WaitForExit(); return url;
                    }
                });

                if (!string.IsNullOrEmpty(streamUrl) && wmp != null) {
                    wmp.URL = streamUrl; wmp.controls.play();
                    btnPlayPause.Text = "⏸"; playbackTimer.Start();
                } else btnPlayPause.Text = "▶";
            } catch { btnPlayPause.Text = "▶"; }
            ForceGarbageCollection();
        }

        private void BtnPlayPause_Click(object sender, EventArgs e) {
            if (wmp == null || currentPlayingTrack == null) return;
            try { if (wmp.playState == 3) { wmp.controls.pause(); btnPlayPause.Text = "▶"; playbackTimer.Stop(); } else { wmp.controls.play(); btnPlayPause.Text = "⏸"; playbackTimer.Start(); } } catch { }
        }

        private void PlaybackTimer_Tick(object sender, EventArgs e) {
            if (wmp == null) return;
            try { 
                if (wmp.playState == 3) { 
                    double pos = wmp.controls.currentPosition; int seconds = (int)pos; 
                    lblCurrentTime.Text = (seconds / 60).ToString() + ":" + (seconds % 60).ToString("00"); 
                    if (seconds <= progressSlider.Maximum) progressSlider.Value = seconds; 
                } 
                else if (wmp.playState == 1) { // 1 = Stopped (Finished)
                    playbackTimer.Stop();
                    PlayNextTrack();
                }
            } catch { }
        }

        private void ForceGarbageCollection() { GC.Collect(); GC.WaitForPendingFinalizers(); GC.Collect(); }

        [STAThread] static void Main() { Application.EnableVisualStyles(); Application.SetCompatibleTextRenderingDefault(false); Application.Run(new PlayerForm()); }
    }
}
