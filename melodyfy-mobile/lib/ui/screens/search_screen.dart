import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../providers/player_provider.dart';
import '../widgets/track_tile.dart';

class SearchScreen extends StatefulWidget {
  const SearchScreen({super.key});

  @override
  State<SearchScreen> createState() => _SearchScreenState();
}

class _SearchScreenState extends State<SearchScreen> {
  final TextEditingController _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit(PlayerProvider provider) {
    final q = _controller.text.trim();
    if (q.isNotEmpty) {
      FocusScope.of(context).unfocus();
      provider.search(q);
    }
  }

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<PlayerProvider>();
    final accent = provider.accentColor;

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 14, 16, 10),
          child: Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _controller,
                  style: const TextStyle(color: Colors.white),
                  textInputAction: TextInputAction.search,
                  onSubmitted: (_) => _submit(provider),
                  decoration: InputDecoration(
                    hintText: '🔍 ¿Qué canción, artista o URL buscas?',
                    hintStyle: TextStyle(color: Colors.grey.shade500),
                    filled: true,
                    fillColor: const Color(0xFF242424),
                    contentPadding:
                        const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(12),
                      borderSide: BorderSide.none,
                    ),
                    suffixIcon: _controller.text.isNotEmpty
                        ? IconButton(
                            icon: const Icon(Icons.close, color: Colors.grey),
                            onPressed: () {
                              _controller.clear();
                              setState(() {});
                            },
                          )
                        : null,
                  ),
                  onChanged: (_) => setState(() {}),
                ),
              ),
              const SizedBox(width: 8),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: accent,
                  foregroundColor: Colors.black,
                  padding:
                      const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
                  shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(12)),
                ),
                onPressed: () => _submit(provider),
                child: const Text('Buscar',
                    style: TextStyle(fontWeight: FontWeight.bold)),
              ),
            ],
          ),
        ),
        SizedBox(
          height: 40,
          child: ListView.separated(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            scrollDirection: Axis.horizontal,
            itemCount: provider.library.quickTags.length,
            separatorBuilder: (_, _) => const SizedBox(width: 8),
            itemBuilder: (_, i) {
              final tag = provider.library.quickTags[i];
              return ActionChip(
                label: Text(tag.label,
                    style:
                        const TextStyle(color: Colors.white, fontSize: 11.5)),
                backgroundColor: const Color(0xFF222222),
                side: BorderSide.none,
                onPressed: () {
                  _controller.text = tag.query;
                  provider.search(tag.query);
                },
              );
            },
          ),
        ),
        const SizedBox(height: 8),
        if (provider.isSearching) LinearProgressIndicator(color: accent),
        Expanded(
          child: provider.searchResults.isEmpty && !provider.isSearching
              ? Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      ClipRRect(
                        borderRadius: BorderRadius.circular(16),
                        child: Image.asset('assets/logo.jpg',
                            width: 90, height: 90, fit: BoxFit.cover),
                      ),
                      const SizedBox(height: 12),
                      const Text('Busca canciones o pega enlaces de YouTube',
                          style: TextStyle(color: Colors.grey)),
                    ],
                  ),
                )
              : ListView.builder(
                  padding: const EdgeInsets.only(bottom: 110),
                  itemCount: provider.searchResults.length,
                  itemBuilder: (_, i) => TrackTile(
                    track: provider.searchResults[i],
                    index: i,
                  ),
                ),
        ),
      ],
    );
  }
}
