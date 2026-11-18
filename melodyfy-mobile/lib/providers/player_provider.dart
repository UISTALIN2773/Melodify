import 'dart:async';
import 'dart:math';
import 'package:audio_service/audio_service.dart';
import 'package:flutter/material.dart';
import '../models/track.dart';
import '../services/audio_handler.dart';
import '../services/download_service.dart';
import '../services/innertube_service.dart';
import '../services/library_service.dart';
import '../services/lyrics_service.dart';

class PlayerProvider extends ChangeNotifier {
  final MelodyfyAudioHandler audioHandler;
  final InnerTubeService innerTube;
  final LyricsService lyricsService;
  final LibraryService library;
  final DownloadService downloader;

  List<Track> searchResults = [];
  List<Track> queue = [];
  int? currentIndex;

  bool isSearching = false;
  bool isLoadingStream = false;
  bool isPlaying = false;
  bool isShuffle = false;
  RepeatMode repeatMode = RepeatMode.all;
  double playbackSpeed = 1.0;

  Duration position = Duration.zero;
  Duration duration = Duration.zero;

  LyricsData? lyricsData;
  bool isLoadingLyrics = false;
  double lyricsOffsetSecs = 0.0;

  int sleepTimerMinutes = 0;
  DateTime? sleepTimerEnd;
  Timer? _sleepCheckTimer;

  String lastQuery = '';
  int popularCategoryFilter = 0;

  void Function(String message)? onToast;

  PlayerProvider({
    required this.audioHandler,
    required this.innerTube,
    required this.lyricsService,
    required this.library,
    required this.downloader,
  }) {
    audioHandler.onTrackCompleted = _handleTrackCompleted;
    audioHandler.onSkipNext = playNext;
    audioHandler.onSkipPrev = playPrev;

    audioHandler.positionStream.listen((pos) {
      position = pos;
      notifyListeners();
    });

    audioHandler.durationStream.listen((dur) {
      if (dur != null) {
        duration = dur;
        notifyListeners();
      }
    });

    audioHandler.playerStateStream.listen((state) {
      isPlaying = state.playing;
      notifyListeners();
    });

    _sleepCheckTimer = Timer.periodic(const Duration(seconds: 5), (_) {
      if (sleepTimerEnd != null && DateTime.now().isAfter(sleepTimerEnd!)) {
        audioHandler.pause();
        sleepTimerEnd = null;
        sleepTimerMinutes = 0;
        showToast('🌙 Temporizador finalizado: Música en pausa');
        notifyListeners();
      }
    });

    // Initial curated home feed
    search('Anime Openings J-Pop Hits');
  }

  Color get accentColor => library.useSpotifyGreen
      ? const Color(0xFF1ED760)
      : const Color(0xFFE87A30);

  Track? get currentTrack {
    if (currentIndex == null) return null;
    if (currentIndex! < 0 || currentIndex! >= queue.length) return null;
    return queue[currentIndex!];
  }

  void showToast(String msg) {
    onToast?.call(msg);
  }

  void adjustLyricsOffset(double delta) {
    lyricsOffsetSecs += delta;
    notifyListeners();
  }

  void resetLyricsOffset() {
    lyricsOffsetSecs = 0.0;
    notifyListeners();
  }

  void toggleThemeAccent() {
    library.useSpotifyGreen = !library.useSpotifyGreen;
    library.save();
    notifyListeners();
  }

  void toggleAutoRadio() {
    library.autoRadio = !library.autoRadio;
    library.save();
    showToast(library.autoRadio
        ? '📻 Radio Infinita ACTIVADA'
        : '📻 Radio Infinita DESACTIVADA');
    notifyListeners();
  }

  Future<void> search(String query) async {
    final q = query.trim();
    if (q.isEmpty) return;
    lastQuery = q;
    isSearching = true;
    notifyListeners();

    final results = await innerTube.search(q);
    searchResults = results;
    isSearching = false;
    notifyListeners();
  }

  Future<void> playTrackNow(Track track) async {
    library.recordHistory(track);
    int idx = queue.indexWhere((t) => t.id == track.id);
    if (idx == -1) {
      queue.add(track);
      idx = queue.length - 1;
    }
    await playQueueIndex(idx);
  }

  Future<void> playQueue(List<Track> tracks, {int startIndex = 0}) async {
    if (tracks.isEmpty) return;
    queue = List<Track>.from(tracks);
    await playQueueIndex(startIndex.clamp(0, queue.length - 1));
  }

  Future<void> playQueueIndex(int index) async {
    if (index < 0 || index >= queue.length) return;
    currentIndex = index;
    final track = queue[index];
    library.recordHistory(track);
    isLoadingStream = true;
    position = Duration.zero;
    duration = Duration(seconds: track.durationSeconds);
    notifyListeners();

    // Prefetch next track
    if (index + 1 < queue.length) {
      innerTube.prefetchStreamUrl(queue[index + 1]);
    }

    // Fetch lyrics in parallel
    fetchLyricsForCurrent();

    try {
      final url = await innerTube.resolveStreamUrl(track);
      if (currentTrack?.id != track.id) return;
      isLoadingStream = false;

      if (url == null || url.isEmpty) {
        showToast('❌ Error al cargar stream de audio');
        notifyListeners();
        return;
      }

      final artUri = track.thumbnailUrl.isNotEmpty
          ? Uri.tryParse(track.thumbnailUrl)
          : null;

      final item = MediaItem(
        id: track.id,
        album: 'Melodyfy Rust Mobile',
        title: track.title,
        artist: track.artist,
        duration: track.durationSeconds > 0
            ? Duration(seconds: track.durationSeconds)
            : null,
        artUri: artUri,
      );

      await audioHandler.playMediaUrl(
        url: url,
        item: item,
        isLocalFile: track.isLocal,
      );
      await audioHandler.setSpeed(playbackSpeed);
      notifyListeners();
    } catch (_) {
      isLoadingStream = false;
      showToast('❌ No se pudo reproducir la pista');
      notifyListeners();
    }
  }

  Future<void> fetchLyricsForCurrent({bool force = false}) async {
    final track = currentTrack;
    if (track == null) return;
    if (!force && lyricsData != null && lyricsData!.trackId == track.id) {
      return;
    }
    lyricsData = null;
    isLoadingLyrics = true;
    notifyListeners();

    final data = await lyricsService.fetchLyrics(track);
    if (currentTrack?.id == track.id) {
      lyricsData = data;
      isLoadingLyrics = false;
      notifyListeners();
    }
  }

  Future<void> togglePlayPause() async {
    if (isPlaying) {
      await audioHandler.pause();
    } else {
      await audioHandler.play();
    }
  }

  Future<void> seek(Duration pos) async {
    await audioHandler.seek(pos);
  }

  Future<void> playNext() async {
    if (queue.isEmpty) {
      if (library.autoRadio) {
        await triggerAutoRadio();
      }
      return;
    }

    if (isShuffle) {
      final rnd = Random().nextInt(queue.length);
      await playQueueIndex(rnd);
      return;
    }

    final curr = currentIndex ?? -1;
    if (curr + 1 < queue.length) {
      await playQueueIndex(curr + 1);
    } else if (library.autoRadio) {
      await triggerAutoRadio();
    } else if (repeatMode == RepeatMode.all) {
      await playQueueIndex(0);
    }
  }

  Future<void> playPrev() async {
    if (position.inSeconds > 3) {
      await seek(Duration.zero);
      return;
    }
    final curr = currentIndex ?? 0;
    if (curr > 0) {
      await playQueueIndex(curr - 1);
    } else if (repeatMode == RepeatMode.all && queue.isNotEmpty) {
      await playQueueIndex(queue.length - 1);
    }
  }

  void _handleTrackCompleted() {
    if (repeatMode == RepeatMode.one) {
      seek(Duration.zero);
      audioHandler.play();
    } else {
      playNext();
    }
  }

  Future<void> triggerAutoRadio() async {
    final seed = currentTrack != null
        ? '${currentTrack!.artist} ${currentTrack!.title} Mix'
        : 'Anime Openings J-Pop Hits';
    showToast('📻 Radio Infinita: Buscando pistas similares...');
    final candidates = await innerTube.search(seed);
    final prevLen = queue.length;
    int added = 0;
    for (final t in candidates) {
      if (!queue.any((e) => e.id == t.id)) {
        queue.add(t);
        added++;
        if (added >= 10) break;
      }
    }
    if (added > 0) {
      showToast('📻 Radio Infinita: +$added pistas añadidas');
      await playQueueIndex(prevLen);
    }
    notifyListeners();
  }

  void toggleShuffle() {
    isShuffle = !isShuffle;
    notifyListeners();
  }

  void cycleRepeatMode() {
    repeatMode = switch (repeatMode) {
      RepeatMode.off => RepeatMode.all,
      RepeatMode.all => RepeatMode.one,
      RepeatMode.one => RepeatMode.off,
    };
    notifyListeners();
  }

  Future<void> cycleSpeed() async {
    if ((playbackSpeed - 1.0).abs() < 0.05) {
      playbackSpeed = 1.25;
      showToast('⚡ Modo Nightcore (1.25x)');
    } else if ((playbackSpeed - 1.25).abs() < 0.05) {
      playbackSpeed = 1.5;
      showToast('⏩ Velocidad Rápida (1.5x)');
    } else if ((playbackSpeed - 1.5).abs() < 0.05) {
      playbackSpeed = 0.85;
      showToast('🌙 Modo Slowed Chill (0.85x)');
    } else {
      playbackSpeed = 1.0;
      showToast('🎵 Velocidad Normal (1.0x)');
    }
    await audioHandler.setSpeed(playbackSpeed);
    notifyListeners();
  }

  void cycleSleepTimer() {
    sleepTimerMinutes = switch (sleepTimerMinutes) {
      0 => 15,
      15 => 30,
      30 => 45,
      45 => 60,
      _ => 0,
    };
    if (sleepTimerMinutes > 0) {
      sleepTimerEnd = DateTime.now().add(Duration(minutes: sleepTimerMinutes));
      showToast('🌙 Temporizador activado: $sleepTimerMinutes min');
    } else {
      sleepTimerEnd = null;
      showToast('🌙 Temporizador desactivado');
    }
    notifyListeners();
  }

  void addToQueue(Track track) {
    queue.add(track);
    showToast('➕ Añadido a la cola');
    notifyListeners();
  }

  void toggleFavorite(Track track) {
    final added = library.toggleFavorite(track);
    showToast(added ? '💚 Guardada en Tus Me Gusta' : 'Eliminada de Favoritos');
    notifyListeners();
  }

  Future<void> downloadTrack(Track track) async {
    showToast('💾 Descargando ${track.title}...');
    final saved = await downloader.downloadTrack(track);
    if (saved != null) {
      showToast('✅ Descarga offline lista: ${track.title}');
    } else {
      showToast('❌ Error al descargar ${track.title}');
    }
    notifyListeners();
  }

  @override
  void dispose() {
    _sleepCheckTimer?.cancel();
    super.dispose();
  }
}
