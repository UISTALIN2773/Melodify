import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../providers/player_provider.dart';
import '../widgets/track_tile.dart';

class HomeScreen extends StatelessWidget {
  final void Function(String query) onTriggerSearch;

  const HomeScreen({super.key, required this.onTriggerSearch});

  static const _curatedMixes = [
    ('🌸 J-Pop Hits', 'J-Pop Hits 2026', Color(0xFFB42D6E)),
    ('⚔️ Anime OPs', 'Anime Openings HD', Color(0xFF2D46B9)),
    ('🎸 Covers OPs', 'Anime Rock Metal Covers', Color(0xFFAF281E)),
    ('🔥 Shonen Hits', 'Shonen Anime Openings', Color(0xFFC36414)),
    ('🌃 City Pop', 'Japanese City Pop 80s', Color(0xFF821E78)),
    ('🎧 Anime Lo-Fi', 'Anime Lofi Chill Beats', Color(0xFF1E7846)),
  ];

  static const _popularCards = [
    ('Idol (アイドル)', 'YOASOBI', 'YOASOBI Idol Official Music Video',
        'https://i.ytimg.com/vi/ZRtdQ81jPUQ/hqdefault.jpg'),
    ('KICK BACK', 'Kenshi Yonezu', 'Kenshi Yonezu KICK BACK Official',
        'https://i.ytimg.com/vi/M2cckDmNLMI/hqdefault.jpg'),
    ('SPECIALZ', 'King Gnu', 'King Gnu SPECIALZ Official Video',
        'https://i.ytimg.com/vi/fhzKLfZq6mg/hqdefault.jpg'),
    ('Bling-Bang-Bang', 'Creepy Nuts', 'Creepy Nuts Bling-Bang-Bang-Born Official',
        'https://i.ytimg.com/vi/210R0ozmLwg/hqdefault.jpg'),
    ('unravel', 'TK • Tokyo Ghoul', 'TK from 凛として時雨 unravel Official',
        'https://i.ytimg.com/vi/Fve_lHIPa-I/hqdefault.jpg'),
    ('Stay With Me', 'Miki Matsubara', 'Miki Matsubara Stay With Me Official Audio',
        'https://i.ytimg.com/vi/9GxP8b0XG5Y/hqdefault.jpg'),
  ];

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<PlayerProvider>();
    final accent = provider.accentColor;

    return CustomScrollView(
      slivers: [
        SliverToBoxAdapter(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 14, 16, 8),
            child: Row(
              children: [
                ClipRRect(
                  borderRadius: BorderRadius.circular(10),
                  child: Image.asset('assets/logo.jpg',
                      width: 42, height: 42, fit: BoxFit.cover),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text(
                        'Buenas tardes',
                        style: TextStyle(
                            color: Colors.white,
                            fontSize: 22,
                            fontWeight: FontWeight.bold),
                      ),
                      Text(
                        'Melodyfy Rust • 0% Anuncios',
                        style: TextStyle(color: accent, fontSize: 11.5),
                      ),
                    ],
                  ),
                ),
                IconButton(
                  icon: Icon(
                    Icons.palette_outlined,
                    color: accent,
                  ),
                  tooltip: 'Cambiar acento Verde/Ámbar',
                  onPressed: provider.toggleThemeAccent,
                ),
              ],
            ),
          ),
        ),
        // Quick Tag Chips
        SliverToBoxAdapter(
          child: SizedBox(
            height: 44,
            child: ListView.separated(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              scrollDirection: Axis.horizontal,
              itemCount: provider.library.quickTags.length,
              separatorBuilder: (_, _) => const SizedBox(width: 8),
              itemBuilder: (_, i) {
                final tag = provider.library.quickTags[i];
                final selected = provider.lastQuery == tag.query;
                return ActionChip(
                  label: Text(
                    tag.label,
                    style: TextStyle(
                      color: selected ? Colors.black : Colors.white,
                      fontWeight: FontWeight.w600,
                      fontSize: 12,
                    ),
                  ),
                  backgroundColor:
                      selected ? accent : const Color(0xFF262626),
                  side: BorderSide.none,
                  shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(20)),
                  onPressed: () => onTriggerSearch(tag.query),
                );
              },
            ),
          ),
        ),
        const SliverToBoxAdapter(child: SizedBox(height: 14)),
        // 2x3 Curated Mix Grid
        SliverPadding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          sliver: SliverGrid(
            gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: 2,
              mainAxisSpacing: 8,
              crossAxisSpacing: 8,
              childAspectRatio: 3.1,
            ),
            delegate: SliverChildBuilderDelegate(
              (_, i) {
                final (title, query, color) = _curatedMixes[i];
                return InkWell(
                  onTap: () => onTriggerSearch(query),
                  borderRadius: BorderRadius.circular(8),
                  child: Container(
                    decoration: BoxDecoration(
                      color: const Color(0xFF242424),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Row(
                      children: [
                        Container(
                          width: 48,
                          height: double.infinity,
                          decoration: BoxDecoration(
                            color: color,
                            borderRadius: const BorderRadius.horizontal(
                                left: Radius.circular(8)),
                          ),
                          child: const Icon(Icons.music_note,
                              color: Colors.white),
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            title,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(
                                color: Colors.white,
                                fontWeight: FontWeight.w600,
                                fontSize: 12),
                          ),
                        ),
                      ],
                    ),
                  ),
                );
              },
              childCount: _curatedMixes.length,
            ),
          ),
        ),
        // Popular Cards Carousel
        const SliverToBoxAdapter(
          child: Padding(
            padding: EdgeInsets.fromLTRB(16, 20, 16, 10),
            child: Text(
              'Populares • Anime & J-Pop',
              style: TextStyle(
                  color: Colors.white,
                  fontSize: 18,
                  fontWeight: FontWeight.bold),
            ),
          ),
        ),
        SliverToBoxAdapter(
          child: SizedBox(
            height: 188,
            child: ListView.separated(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              scrollDirection: Axis.horizontal,
              itemCount: _popularCards.length,
              separatorBuilder: (_, _) => const SizedBox(width: 12),
              itemBuilder: (_, i) {
                final (title, artist, query, thumb) = _popularCards[i];
                return GestureDetector(
                  onTap: () => onTriggerSearch(query),
                  child: SizedBox(
                    width: 132,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        ClipRRect(
                          borderRadius: BorderRadius.circular(10),
                          child: CachedNetworkImage(
                            imageUrl: thumb,
                            width: 132,
                            height: 132,
                            fit: BoxFit.cover,
                            errorWidget: (_, _, _) => Image.asset(
                                'assets/logo.jpg',
                                width: 132,
                                height: 132,
                                fit: BoxFit.cover),
                          ),
                        ),
                        const SizedBox(height: 6),
                        Text(
                          title,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(
                              color: Colors.white,
                              fontWeight: FontWeight.w600,
                              fontSize: 13),
                        ),
                        Text(
                          artist,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                              color: Colors.grey.shade400, fontSize: 11),
                        ),
                      ],
                    ),
                  ),
                );
              },
            ),
          ),
        ),
        const SliverToBoxAdapter(
          child: Padding(
            padding: EdgeInsets.fromLTRB(16, 16, 16, 6),
            child: Text(
              'Recomendado para ti',
              style: TextStyle(
                  color: Colors.white,
                  fontSize: 18,
                  fontWeight: FontWeight.bold),
            ),
          ),
        ),
        if (provider.isSearching)
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Center(child: CircularProgressIndicator(color: accent)),
            ),
          )
        else
          SliverList(
            delegate: SliverChildBuilderDelegate(
              (_, i) => TrackTile(
                track: provider.searchResults[i],
                index: i,
              ),
              childCount: provider.searchResults.length.clamp(0, 15),
            ),
          ),
        const SliverToBoxAdapter(child: SizedBox(height: 110)),
      ],
    );
  }
}
