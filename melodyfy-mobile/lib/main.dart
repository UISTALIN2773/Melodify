import 'package:audio_service/audio_service.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'providers/player_provider.dart';
import 'services/audio_handler.dart';
import 'services/download_service.dart';
import 'services/innertube_service.dart';
import 'services/library_service.dart';
import 'services/lyrics_service.dart';
import 'ui/screens/home_screen.dart';
import 'ui/screens/library_screen.dart';
import 'ui/screens/lyrics_screen.dart';
import 'ui/screens/search_screen.dart';
import 'ui/widgets/mini_player.dart';

late MelodyfyAudioHandler _audioHandler;

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  _audioHandler = await AudioService.init(
    builder: () => MelodyfyAudioHandler(),
    config: const AudioServiceConfig(
      androidNotificationChannelId: 'com.melodyfy.melodyfy_mobile.audio',
      androidNotificationChannelName: 'Melodyfy Reproducción',
      androidNotificationOngoing: true,
      androidStopForegroundOnPause: true,
    ),
  );

  final library = LibraryService();
  await library.load();

  final innerTube = InnerTubeService();
  final lyrics = LyricsService();
  final downloader = DownloadService(innerTube: innerTube, library: library);

  runApp(
    ChangeNotifierProvider(
      create: (_) => PlayerProvider(
        audioHandler: _audioHandler,
        innerTube: innerTube,
        lyricsService: lyrics,
        library: library,
        downloader: downloader,
      ),
      child: const MelodyfyMobileApp(),
    ),
  );
}

class MelodyfyMobileApp extends StatelessWidget {
  const MelodyfyMobileApp({super.key});

  @override
  Widget build(BuildContext context) {
    final accent = context.select<PlayerProvider, Color>((p) => p.accentColor);
    return MaterialApp(
      title: 'Melodyfy Rust Mobile',
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF121212),
        colorScheme: ColorScheme.dark(primary: accent, secondary: accent),
        appBarTheme: const AppBarTheme(backgroundColor: Color(0xFF121212)),
      ),
      home: const _MainShell(),
    );
  }
}

class _MainShell extends StatefulWidget {
  const _MainShell();

  @override
  State<_MainShell> createState() => _MainShellState();
}

class _MainShellState extends State<_MainShell> {
  int _tabIndex = 0;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final provider = context.read<PlayerProvider>();
      provider.onToast = (msg) {
        if (!mounted) return;
        ScaffoldMessenger.of(context)
          ..hideCurrentSnackBar()
          ..showSnackBar(
            SnackBar(
              content: Text(msg,
                  style: const TextStyle(
                      color: Colors.black, fontWeight: FontWeight.bold)),
              backgroundColor: provider.accentColor,
              behavior: SnackBarBehavior.floating,
              duration: const Duration(seconds: 2),
            ),
          );
      };
    });
  }

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<PlayerProvider>();
    final accent = provider.accentColor;

    final screens = [
      HomeScreen(
        onTriggerSearch: (query) {
          provider.search(query);
          setState(() => _tabIndex = 1);
        },
      ),
      const SearchScreen(),
      const LibraryScreen(),
      const LyricsScreen(),
    ];

    return Scaffold(
      backgroundColor: const Color(0xFF121212),
      body: SafeArea(
        child: Stack(
          children: [
            IndexedStack(
              index: _tabIndex,
              children: screens,
            ),
            const Positioned(
              left: 0,
              right: 0,
              bottom: 0,
              child: MiniPlayer(),
            ),
          ],
        ),
      ),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _tabIndex,
        onTap: (idx) {
          setState(() => _tabIndex = idx);
          if (idx == 3) {
            provider.fetchLyricsForCurrent();
          }
        },
        type: BottomNavigationBarType.fixed,
        backgroundColor: const Color(0xFF181818),
        selectedItemColor: accent,
        unselectedItemColor: Colors.grey,
        selectedFontSize: 11,
        unselectedFontSize: 11,
        items: const [
          BottomNavigationBarItem(
              icon: Icon(Icons.home_rounded), label: 'Inicio'),
          BottomNavigationBarItem(
              icon: Icon(Icons.search_rounded), label: 'Buscar'),
          BottomNavigationBarItem(
              icon: Icon(Icons.library_music_rounded), label: 'Biblioteca'),
          BottomNavigationBarItem(
              icon: Icon(Icons.mic_rounded), label: 'Karaoke'),
        ],
      ),
    );
  }
}
