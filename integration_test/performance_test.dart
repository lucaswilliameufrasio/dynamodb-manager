import 'package:dynamodb_manager/src/models/dynamo_item.dart';
import 'package:dynamodb_manager/src/widgets/dynamo_items_list.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  final items = List.generate(
    50,
    (index) => DynamoItem.fromDynamoJson(
      '{"pk":"item-$index","status":"pending","amount":${index * 5}}',
    ),
  );

  testWidgets('profile scrolling a 50-item DynamoDB page', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: DynamoItemsList(
            itemsProvider: () => List.unmodifiable(items),
            selectedItemProvider: () => null,
            onSelect: (_) {},
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await binding.watchPerformance(() async {
      final list = find.byType(ListView);
      for (var i = 0; i < 12; i++) {
        await tester.drag(list, const Offset(0, -480));
        await tester.pumpAndSettle();
        await tester.drag(list, const Offset(0, 480));
        await tester.pumpAndSettle();
      }
    }, reportKey: 'dynamodb_items_50_scroll');

    expect(find.text('pk: item-0'), findsOneWidget);
  });
}
