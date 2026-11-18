import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../models/track.dart';
import '../../providers/player_provider.dart';
import '../widgets/spectrum_visualizer.dart';
import 'lyrics_screen.dart';

class PlayerScreen extends StatelessWidget {
  const PlayerScreen({super.key});

  String _fmt(Duration d) {
    final secs = d.inSeconds;
    return '${secs ~/ 60}:${(secs % 60).toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<PlayerProvider>();
    final track = provider.currentTrack;
    final accent = provider.accentColor;

    final durSecs = provider.duration.inSeconds > 0
        ? provider.duration.inSeconds.toDouble()
        : (track?.durationSeconds.toDouble() ?? 100.0);
    final posSecs =
        provider.position.inSeconds.toDouble().clamp(0.0, durSecs > 0 ? durSecs : 100.0);
    final isFav = track != null && provider.library.isFavorite(track.id);

    return Scaffold(
      backgroundColor: const Color(0xFF121212),
      body: SafeArea(
        child: GestureDetector(
          onVerticalDragEnd: (details) {
            if ((details.primaryVelocity ?? 0) > 400) {
              Navigator.pop(context);
            }
          },
          onHorizontalDragEnd: (details) {
            if ((details.primaryVelocity ?? 0) < -350) {
              provider.playNext();
            } else if ((details.primaryVelocity ?? 0) > 350) {
              provider.playPrev();
            }
          },
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 22),
            child: Column(
              children: [
                const SizedBox(height: 6),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    IconButton(
                      icon: const Icon(Icons.keyboard_arrow_down_rounded,
                          color: Colors.white, size: 32),
                      onPressed: () => Navigator.pop(context),
                    ),
                    Column(
                      children: [
                        Text('REPRODUCIENDO',
                            style: TextStyle(
                                color: Colors.grey.shade400,
                                fontSize: 10,
                                letterSpacing: 1.4)),
                        const Text('Melodyfy Rust Mobile',
                            style: TextStyle(
                                color: Colors.white,
                                fontWeight: FontWeight.bold,
                                fontSize: 13)),
                      ],
                    ),
                    IconButton(
                      icon: ClipRRect(
                        borderRadius: BorderRadius.circular(6),
                        child: Image.asset('assets/logo.jpg',
                            width: 28, height: 28, fit: BoxFit.cover),
                      ),
                      onPressed: () => provider.toggleThemeAccent(),
                    ),
                  ],
                ),
                const Spacer(),
                ClipRRect(
                  borderRadius: BorderRadius.circular(18),
                  child: Container(
                    width: 290,
                    height: 290,
                    decoration: BoxDecoration(
                      color: const Color(0xFF1E1E1E),
                      boxShadow: [
                        BoxShadow(
                          color: accent.withValues(alpha: 0.18),
                          blurRadius: 32,
                          spreadRadius: 2,
                        )
                      ],
                    ),
                    child: track != null && track.thumbnailUrl.isNotEmpty
                        ? CachedNetworkImage(
                            imageUrl: track.thumbnailUrl,
                            fit: BoxFit.cover,
                            placeholder: (_, _) =>
                                Image.asset('assets/logo.jpg', fit: BoxFit.cover),
                            errorWidget: (_, _, _) =>
                                Image.asset('assets/logo.jpg', fit: BoxFit.cover),
                          )
                        : Image.asset('assets/logo.jpg', fit: BoxFit.cover),
                  ),
                ),
                const SizedBox(height: 18),
                if (provider.isPlaying)
                  SpectrumVisualizer(
                    isPlaying: provider.isPlaying,
                    color: accent,
                    barCount: 16,
                    height: 18,
                    barWidth: 4,
                  ),
                const Spacer(),
                Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            track?.title ?? 'Elige una canción',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(
                              color: Colors.white,
                              fontSize: 20,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                          const SizedBox(height: 4),
                          Text(
                            track?.artist ?? 'Melodyfy Audio Engine',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: TextStyle(
                              color: Colors.grey.shade400,
                              fontSize: 14,
                            ),
                          ),
                        ],
                      ),
                    ),
                    if (track != null)
                      IconButton(
                        icon: Icon(
                          isFav ? Icons.favorite : Icons.favorite_border,
                          color: isFav ? accent : Colors.white70,
                          size: 28,
                        ),
                        onPressed: () => provider.toggleFavorite(track),
                      ),
                  ],
                ),
                const SizedBox(height: 12),
                SliderTheme(
                  data: SliderTheme.of(context).copyWith(
                    trackHeight: 4,
                    thumbShape:
                        const RoundSliderThumbShape(enabledThumbRadius: 6),
                    activeTrackColor: accent,
                    inactiveTrackColor: Colors.white24,
                    thumbColor: Colors.white,
                  ),
                  child: Slider(
                    value: posSecs,
                    min: 0.0,
                    max: durSecs > 0 ? durSecs : 100.0,
                    onChanged: (val) =>
                        provider.seek(Duration(seconds: val.round())),
                  ),
                ),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(_fmt(provider.position),
                        style: TextStyle(
                            color: Colors.grey.shade400, fontSize: 12)),
                    Text(
                      durSecs > 0
                          ? _fmt(Duration(seconds: durSecs.round()))
                          : (track?.duration ?? '0:00'),
                      style:
                          TextStyle(color: Colors.grey.shade400, fontSize: 12),
                    ),
                  ],
                ),
                const SizedBox(height: 14),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    IconButton(
                      icon: Icon(
                        Icons.shuffle_rounded,
                        color: provider.isShuffle ? accent : Colors.white54,
                        size: 26,
                      ),
                      onPressed: provider.toggleShuffle,
                    ),
                    IconButton(
                      icon: const Icon(Icons.skip_previous_rounded,
                          color: Colors.white, size: 38),
                      onPressed: provider.playPrev,
                    ),
                    GestureDetector(
                      onTap: provider.togglePlayPause,
                      child: Container(
                        width: 68,
                        height: 68,
                        decoration: BoxDecoration(
                          color: accent,
                          shape: BoxShape.circle,
                        ),
                        child: Icon(
                          provider.isPlaying
                              ? Icons.pause_rounded
                              : Icons.play_arrow_rounded,
                          color: Colors.black,
                          size: 38,
                        ),
                      ),
                    ),
                    IconButton(
                      icon: const Icon(Icons.skip_next_rounded,
                          color: Colors.white, size: 38),
                      onPressed: provider.playNext,
                    ),
                    IconButton(
                      icon: Icon(
                        provider.repeatMode == RepeatMode.one
                            ? Icons.repeat_one_rounded
                            : Icons.repeat_rounded,
                        color: provider.repeatMode == RepeatMode.off
                            ? Colors.white54
                            : accent,
                        size: 26,
                      ),
                      onPressed: provider.cycleRepeatMode,
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                // DSP Speed, Radio, Sleep Timer, Download, Karaoke Row
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceAround,
                  children: [
                    _toolChip(
                      label:
                          '${provider.playbackSpeed.toStringAsFixed(2).replaceAll('.00', '.0')}x',
                      icon: Icons.speed,
                      active: (provider.playbackSpeed - 1.0).abs() > 0.05,
                      accent: accent,
                      onTap: provider.cycleSpeed,
                    ),
                    _toolChip(
                      label: 'Radio',
                      icon: Icons.radio,
                      active: provider.library.autoRadio,
                      accent: accent,
                      onTap: provider.toggleAutoRadio,
                    ),
                    _toolChip(
                      label: provider.sleepTimerMinutes > 0
                          ? '${provider.sleepTimerMinutes}m'
                          : 'Timer',
                      icon: Icons.nightlight_round,
                      active: provider.sleepTimerMinutes > 0,
                      accent: accent,
                      onTap: provider.cycleSleepTimer,
                    ),
                    _toolChip(
                      label: 'Letra',
                      icon: Icons.mic_external_on_rounded,
                      active: false,
                      accent: accent,
                      onTap: () {
                        Navigator.of(context).push(
                          MaterialPageRoute(
                            builder: (_) => const Scaffold(
                              backgroundColor: Color(0xFF121212),
                              body: SafeArea(child: LyricsScreen()),
                            ),
                          ),
                        );
                      },
                    ),
                    if (track != null && !track.isLocal)
                      _toolChip(
                        label: 'Bajar',
                        icon: Icons.download_rounded,
                        active: false,
                        accent: accent,
                        onTap: () => provider.downloadTrack(track),
                      ),
                  ],
                ),
                const SizedBox(height: 16),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _toolChip({
    required String label,
    required IconData icon,
    required bool active,
    required Color accent,
    required VoidCallback onTap,
  }) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(14),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
        decoration: BoxDecoration(
          color: active ? accent.withValues(alpha: 0.18) : const Color(0xFF222222),
          borderRadius: BorderRadius.circular(14),
          border: Border.all(color: active ? accent : Colors.white10),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 15, color: active ? accent : Colors.white70),
            const SizedBox(width: 4),
            Text(
              label,
              style: TextStyle(
                color: active ? accent : Colors.white70,
                fontSize: 11.5,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
